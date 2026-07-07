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

//! Address / port enrichment: label every address in the model with the
//! network **zone** and **CIDR name** it falls in (from the manifest DSL's
//! `zone` / `cidr-name` declarations, carried on the `architecture` object),
//! and every port with its service name (the built-in F5 service table).
//!
//! The result is a compact `enrichment` block on the model —
//! `{addresses: {addr: {zone?, cidrName?}}, ports: {port: service}}` — that the
//! front-end looks up to annotate destinations, members, nodes and listeners.
//! DNS / NAT file enrichment (zone files, NAT CSV) layers on top of this via
//! the query engine's side-inputs.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::str::FromStr;

use ipnet::IpNet;
use serde_json::{Map, Value as J};

use crate::jutil::{barr, bstr};
use crate::services::getservbyport;

/// Normalise an address token to a bare `IpAddr` (strip `%route-domain`, a
/// `/prefix`, and an IPv4 `:port`); `None` for hostnames / wildcards.
fn to_ip(addr: &str) -> Option<IpAddr> {
    let a = addr.trim();
    if a.is_empty() {
        return None;
    }
    let a = a.split('%').next().unwrap_or(a);
    let a = a.split('/').next().unwrap_or(a);
    if let Ok(ip) = IpAddr::from_str(a) {
        return Some(ip);
    }
    // IPv4 `host:port`.
    if let Some((host, _)) = a.rsplit_once(':')
        && !host.contains(':')
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Some(ip);
    }
    None
}

/// Does `net` contain `ip`, honouring IPv4-in-IPv6 (an IPv4 address is inside an
/// IPv6 zone covering its `::ffff:0:0/96`-mapped form).
fn net_contains(net: &IpNet, ip: IpAddr) -> bool {
    if net.contains(&ip) {
        return true;
    }
    match (net, ip) {
        (IpNet::V6(_), IpAddr::V4(v4)) => net.contains(&IpAddr::V6(v4.to_ipv6_mapped())),
        (IpNet::V4(_), IpAddr::V6(v6)) => {
            // A v4-mapped v6 address vs a v4 zone.
            v6.to_ipv4_mapped()
                .is_some_and(|v4| net.contains(&IpAddr::V4(v4)))
        }
        _ => false,
    }
}

/// `[(net, label)]` pairs, most-specific first (longer prefix wins).
fn parse_nets(items: &[(String, String)]) -> Vec<(IpNet, String)> {
    let mut out: Vec<(IpNet, String)> = items
        .iter()
        .filter_map(|(cidr, label)| {
            let net = IpNet::from_str(cidr)
                .ok()
                .or_else(|| IpAddr::from_str(cidr).ok().map(IpNet::from))?;
            Some((net, label.clone()))
        })
        .collect();
    out.sort_by(|a, b| b.0.prefix_len().cmp(&a.0.prefix_len()));
    out
}

/// Collect the distinct addresses and ports referenced across every device.
fn collect_addrs_ports(devices: &[J]) -> (Vec<String>, Vec<i64>) {
    let mut addrs: BTreeMap<String, ()> = BTreeMap::new();
    let mut ports: BTreeMap<i64, ()> = BTreeMap::new();
    {
        let mut add_addr = |s: &str| {
            if to_ip(s).is_some() {
                addrs.insert(s.trim().to_string(), ());
            }
        };
        let mut add_port = |s: &str| {
            if let Ok(p) = s.trim().parse::<i64>() {
                ports.insert(p, ());
            }
        };
        for d in devices {
            let Some(dm) = d.as_object() else { continue };
            for v in barr(dm, "virtuals") {
                if let Some(m) = v.as_object() {
                    add_addr(bstr(m, "destAddr"));
                    add_port(bstr(m, "destPort"));
                }
            }
            for p in barr(dm, "pools") {
                let Some(pm) = p.as_object() else { continue };
                for mem in barr(pm, "members") {
                    if let Some(mm) = mem.as_object() {
                        add_addr(bstr(mm, "address"));
                        add_port(bstr(mm, "port"));
                    }
                }
            }
            for n in barr(dm, "nodes") {
                if let Some(nm) = n.as_object() {
                    add_addr(bstr(nm, "address"));
                }
            }
        }
    }
    (addrs.into_keys().collect(), ports.into_keys().collect())
}

/// Build the `enrichment` model block from the model devices and the already-
/// computed `architecture` object (which carries the DSL's zones + cidr-names).
#[must_use]
pub fn build_enrichment(devices: &[J], architecture: &J) -> J {
    // Zones → nets.
    let arch = architecture.as_object();
    let mut zone_nets: Vec<(IpNet, String)> = Vec::new();
    if let Some(zones) = arch.and_then(|a| a.get("zones")).and_then(J::as_array) {
        for z in zones {
            let Some(zm) = z.as_object() else { continue };
            let name = bstr(zm, "name").to_string();
            for c in barr(zm, "cidrs") {
                if let Some(cs) = c.as_str()
                    && let Some(net) = IpNet::from_str(cs)
                        .ok()
                        .or_else(|| IpAddr::from_str(cs).ok().map(IpNet::from))
                {
                    zone_nets.push((net, name.clone()));
                }
            }
        }
    }
    zone_nets.sort_by(|a, b| b.0.prefix_len().cmp(&a.0.prefix_len()));

    // cidr-name entries.
    let cidr_pairs: Vec<(String, String)> = arch
        .and_then(|a| a.get("maps"))
        .and_then(J::as_object)
        .and_then(|m| m.get("cidrNames"))
        .and_then(J::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let em = e.as_object()?;
                    Some((bstr(em, "cidr").to_string(), bstr(em, "label").to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    let cidr_nets = parse_nets(&cidr_pairs);

    let (addrs, ports) = collect_addrs_ports(devices);

    let mut addr_map = Map::new();
    for a in addrs {
        let Some(ip) = to_ip(&a) else { continue };
        let zone = zone_nets.iter().find(|(n, _)| net_contains(n, ip)).map(|(_, l)| l);
        let cidr_name = cidr_nets.iter().find(|(n, _)| net_contains(n, ip)).map(|(_, l)| l);
        if zone.is_none() && cidr_name.is_none() {
            continue;
        }
        let mut o = Map::new();
        if let Some(z) = zone {
            o.insert("zone".into(), J::String(z.clone()));
        }
        if let Some(c) = cidr_name {
            o.insert("cidrName".into(), J::String(c.clone()));
        }
        addr_map.insert(a, J::Object(o));
    }

    let mut port_map = Map::new();
    for p in ports {
        if let Some(svc) = getservbyport(p) {
            port_map.insert(p.to_string(), J::String(svc.to_string()));
        }
    }

    let mut out = Map::new();
    out.insert("addresses".into(), J::Object(addr_map));
    out.insert("ports".into(), J::Object(port_map));
    J::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_addresses_and_ports() {
        let devices = vec![serde_json::json!({
            "virtuals": [{"destAddr": "192.0.2.10", "destPort": "443"}],
            "pools": [{"members": [{"address": "10.1.2.3", "port": "80"}]}],
            "nodes": [{"address": "10.1.2.3"}],
        })];
        let arch = serde_json::json!({
            "zones": [{"name": "dmz", "cidrs": ["192.0.2.0/24"]},
                      {"name": "internal", "cidrs": ["10.0.0.0/8"]}],
            "maps": {"cidrNames": [{"cidr": "10.1.0.0/16", "label": "Datacenter A"}]},
        });
        let e = build_enrichment(&devices, &arch);
        let o = e.as_object().unwrap();
        let addrs = o["addresses"].as_object().unwrap();
        assert_eq!(addrs["192.0.2.10"]["zone"], "dmz");
        assert_eq!(addrs["10.1.2.3"]["zone"], "internal");
        assert_eq!(addrs["10.1.2.3"]["cidrName"], "Datacenter A");
        let ports = o["ports"].as_object().unwrap();
        assert_eq!(ports["443"], "https");
        assert_eq!(ports["80"], "http");
    }

    #[test]
    fn ipv4_in_ipv6_zone() {
        // An IPv4 address falls inside an IPv6 zone covering the v4-mapped range.
        let devices = vec![serde_json::json!({"nodes": [{"address": "203.0.113.5"}]})];
        let arch = serde_json::json!({"zones": [{"name": "mapped", "cidrs": ["::ffff:0:0/96"]}]});
        let e = build_enrichment(&devices, &arch);
        assert_eq!(e["addresses"]["203.0.113.5"]["zone"], "mapped");
    }
}
