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

//! Net / IP / CIDR-category builtins, plus the typed address / port /
//! folder value layer they build on.
//!
//! These are the **pure** string/address transforms: destination /
//! address / network / route-domain / partition / folder parsing, the IP
//! classification predicates, CIDR algebra (`in_cidr`, `collapse_cidrs`,
//! `supernet_of`, `subnet_of`, `overlaps`), port-set + IP-range helpers,
//! and the path edit helpers (`with_name`, `in_partition`, `in_folder`,
//! `can_see`).
//!
//! Behaviour notes:
//! - IP parsing + canonical string forms route through `std::net`
//!   (`Ipv4Addr` / `Ipv6Addr`), whose `Display` produces the canonical
//!   zero-run-compressed form bit-for-bit (including IPv4-mapped dotted
//!   form). Network canonicalisation uses `ipnet`.
//! - Address **classification** (`is_private` / `is_global` / `is_reserved`
//!   / …) is implemented against the exact IANA special-purpose-address range
//!   tables, because std's equivalents are unstable on
//!   stable Rust and would not build under `unsafe_code = "forbid"`.
//! - `collapse_cidrs` collapses adjacent / overlapping CIDRs; range
//!   summarisation turns an address range into a minimal CIDR list.
//! - Error message text (including the interpolated address / network
//!   `ValueError` strings, e.g. `'x' does not appear to be an IPv4 or IPv6
//!   address`) is reproduced verbatim.

use std::net::{IpAddr, Ipv4Addr};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use crate::builtins::{BuiltinSpec, plain, type_name};
use crate::errors::QueryError;
use crate::value::Value;

mod address_builtins;
mod cidr_builtins;
mod classification;
mod path_builtins;
mod range_builtins;
mod typed_values;

use address_builtins::{
    bi_host, bi_in_cidr, bi_ip, bi_ip_translate, bi_is_documentation, bi_is_fqdn, bi_is_ipv4,
    bi_is_ipv6, bi_is_link_local, bi_is_loopback, bi_is_multicast, bi_is_private, bi_is_public,
    bi_is_reserved, bi_is_unspecified, bi_is_wildcard_port, bi_net, bi_port, bi_route_domain,
    bi_with_route_domain,
};
use cidr_builtins::{
    bi_broadcast_address, bi_collapse_cidrs, bi_first_host, bi_host_count, bi_last_host,
    bi_network_address, bi_overlaps, bi_prefix_length, bi_subnet_of, bi_supernet_of,
};
use path_builtins::{
    bi_can_see, bi_folder, bi_in_folder, bi_in_partition, bi_with_folder, bi_with_host,
    bi_with_name, bi_with_port,
};
use range_builtins::{
    bi_ip_range_contains, bi_ip_range_count, bi_ip_range_supernet, bi_ip_range_to_cidrs,
    bi_port_set_contains, bi_port_set_count, bi_port_set_overlaps,
};
#[cfg(test)]
use typed_values::Destination;

// A flat registration table — one `plain(...)` row per builtin.
#[allow(clippy::too_many_lines)]
pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("ip", "net", 1, Some(2), false, bi_ip),
        plain("ip_translate", "net", 3, Some(3), false, bi_ip_translate),
        plain("net", "net", 1, Some(1), false, bi_net),
        plain("host", "net", 1, Some(1), false, bi_host),
        plain("port", "net", 1, Some(1), false, bi_port),
        plain("in_cidr", "net", 2, Some(2), false, bi_in_cidr),
        plain("route_domain", "net", 1, Some(1), false, bi_route_domain),
        plain(
            "with_route_domain",
            "net",
            2,
            Some(2),
            false,
            bi_with_route_domain,
        ),
        plain("is_ipv4", "net", 1, Some(1), false, bi_is_ipv4),
        plain("is_ipv6", "net", 1, Some(1), false, bi_is_ipv6),
        plain("is_fqdn", "net", 1, Some(1), false, bi_is_fqdn),
        plain("is_private", "net", 1, Some(1), false, bi_is_private),
        plain("is_loopback", "net", 1, Some(1), false, bi_is_loopback),
        plain(
            "is_unspecified",
            "net",
            1,
            Some(1),
            false,
            bi_is_unspecified,
        ),
        plain("is_multicast", "net", 1, Some(1), false, bi_is_multicast),
        plain("is_link_local", "net", 1, Some(1), false, bi_is_link_local),
        plain("is_reserved", "net", 1, Some(1), false, bi_is_reserved),
        plain("is_public", "net", 1, Some(1), false, bi_is_public),
        plain(
            "is_documentation",
            "net",
            1,
            Some(1),
            false,
            bi_is_documentation,
        ),
        plain(
            "is_wildcard_port",
            "net",
            1,
            Some(1),
            false,
            bi_is_wildcard_port,
        ),
        plain("prefix_length", "net", 1, Some(1), false, bi_prefix_length),
        plain(
            "network_address",
            "net",
            1,
            Some(1),
            false,
            bi_network_address,
        ),
        plain(
            "broadcast_address",
            "net",
            1,
            Some(1),
            false,
            bi_broadcast_address,
        ),
        plain("first_host", "net", 1, Some(1), false, bi_first_host),
        plain("last_host", "net", 1, Some(1), false, bi_last_host),
        plain("host_count", "net", 1, Some(1), false, bi_host_count),
        plain("collapse_cidrs", "net", 1, Some(1), true, bi_collapse_cidrs),
        plain("supernet_of", "net", 1, Some(1), true, bi_supernet_of),
        plain("subnet_of", "net", 2, Some(2), false, bi_subnet_of),
        plain("overlaps", "net", 2, Some(2), false, bi_overlaps),
        plain("with_port", "net", 2, Some(2), false, bi_with_port),
        plain("with_host", "net", 2, Some(2), false, bi_with_host),
        plain("folder", "net", 1, Some(1), false, bi_folder),
        plain("with_folder", "net", 2, Some(2), false, bi_with_folder),
        plain("with_name", "net", 2, Some(2), false, bi_with_name),
        plain("in_partition", "net", 2, Some(2), false, bi_in_partition),
        plain("in_folder", "net", 2, Some(2), false, bi_in_folder),
        plain("can_see", "net", 2, Some(2), false, bi_can_see),
        plain(
            "port_set_contains",
            "net",
            2,
            Some(2),
            false,
            bi_port_set_contains,
        ),
        plain(
            "port_set_count",
            "net",
            1,
            Some(1),
            false,
            bi_port_set_count,
        ),
        plain(
            "port_set_overlaps",
            "net",
            2,
            Some(2),
            false,
            bi_port_set_overlaps,
        ),
        plain(
            "ip_range_to_cidrs",
            "net",
            1,
            Some(1),
            false,
            bi_ip_range_to_cidrs,
        ),
        plain(
            "ip_range_supernet",
            "net",
            1,
            Some(1),
            false,
            bi_ip_range_supernet,
        ),
        plain(
            "ip_range_count",
            "net",
            1,
            Some(1),
            false,
            bi_ip_range_count,
        ),
        plain(
            "ip_range_contains",
            "net",
            2,
            Some(2),
            false,
            bi_ip_range_contains,
        ),
    ]
}

// Shared coercion helpers

/// Coerce a path-like value: accepts string / path-ref / object.
fn coerce_pathlike(v: &Value, name: &str, arg: usize) -> Result<String, QueryError> {
    match v {
        Value::Str(s) => Ok(s.clone()),
        Value::PathRef(p) => Ok(p.full_path.clone()),
        Value::ObjectRef(o) => Ok(o.full_path.clone()),
        other => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be a path-string or object, got {}",
            type_name(other)
        ))),
    }
}

/// Repr-style rendering of a string (single-quoted). The inputs
/// we interpolate are address tokens — plain ASCII without quotes — so a simple
/// single-quote wrap matches the repr quoting for the cases we hit.
fn py_str_repr(s: &str) -> String {
    // Repr quoting prefers single quotes unless the string contains a single
    // quote and no double quote.
    if s.contains('\'') && !s.contains('"') {
        format!("\"{s}\"")
    } else {
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }
}

/// The address-parse `ValueError` text.
fn addr_value_error(text: &str) -> String {
    format!(
        "{} does not appear to be an IPv4 or IPv6 address",
        py_str_repr(text)
    )
}

/// The network-parse `ValueError` text.
fn net_value_error(text: &str) -> String {
    format!(
        "{} does not appear to be an IPv4 or IPv6 network",
        py_str_repr(text)
    )
}

// IP address parsing + classification

/// Parse a bare host (v4 or v6).
/// Parsing is strict: it rejects leading zeros in IPv4 octets, zone ids, etc.
/// `std`'s parser matches on the shapes we care about.
fn parse_ip_address(text: &str) -> Option<IpAddr> {
    // Address parsing does not strip whitespace; callers strip first.
    text.parse::<IpAddr>().ok()
}

/// A parsed network, from a non-strict network parse (host bits allowed).
#[derive(Clone, Copy)]
enum Net {
    V4(Ipv4Net),
    V6(Ipv6Net),
}

impl Net {
    fn network_str(self) -> String {
        match self {
            Net::V4(n) => format!("{}/{}", n.network(), n.prefix_len()),
            Net::V6(n) => format!("{}/{}", n.network(), n.prefix_len()),
        }
    }

    fn is_v4(self) -> bool {
        matches!(self, Net::V4(_))
    }

    fn prefix_len(self) -> u8 {
        match self {
            Net::V4(n) => n.prefix_len(),
            Net::V6(n) => n.prefix_len(),
        }
    }

    fn network_int(self) -> u128 {
        match self {
            Net::V4(n) => u128::from(u32::from(n.network())),
            Net::V6(n) => u128::from(n.network()),
        }
    }

    fn broadcast_int(self) -> u128 {
        match self {
            Net::V4(n) => u128::from(u32::from(n.broadcast())),
            Net::V6(n) => u128::from(n.broadcast()),
        }
    }

    fn network_address_str(self) -> String {
        match self {
            Net::V4(n) => n.network().to_string(),
            Net::V6(n) => n.network().to_string(),
        }
    }

    fn broadcast_address_str(self) -> String {
        match self {
            Net::V4(n) => n.broadcast().to_string(),
            Net::V6(n) => n.broadcast().to_string(),
        }
    }

    fn num_addresses(self) -> u128 {
        let host_bits = u32::from(self.max_prefixlen() - self.prefix_len());
        1u128 << host_bits
    }

    fn max_prefixlen(self) -> u8 {
        if self.is_v4() { 32 } else { 128 }
    }
}

/// Parse a CIDR network non-strictly, accepting integer-prefix and
/// (IPv4) dotted-quad-netmask spellings.
fn parse_network(text: &str) -> Result<Net, String> {
    // `ipnet` already accepts `addr/prefixlen` and masks host bits via
    // `.trunc()`. It does NOT accept dotted-quad netmasks, so handle those.
    let trimmed = text;
    if let Some((addr_part, mask_part)) = trimmed.split_once('/') {
        // IPv4 dotted-quad netmask form: `10.0.0.0/255.255.255.0`.
        if mask_part.contains('.') {
            if let (Ok(addr), Ok(mask)) =
                (addr_part.parse::<Ipv4Addr>(), mask_part.parse::<Ipv4Addr>())
                && let Some(prefix) = netmask_to_prefix(u32::from(mask))
            {
                let net = Ipv4Net::new(addr, prefix).map_err(|_| net_value_error(text))?;
                return Ok(Net::V4(net.trunc()));
            }
            return Err(net_value_error(text));
        }
    }
    match trimmed.parse::<IpNet>() {
        Ok(IpNet::V4(n)) => Ok(Net::V4(n.trunc())),
        Ok(IpNet::V6(n)) => Ok(Net::V6(n.trunc())),
        Err(_) => {
            // A bare address with no prefix is a valid `ip_network` (host
            // network /32 or /128).
            match trimmed.parse::<IpAddr>() {
                Ok(IpAddr::V4(a)) => Ok(Net::V4(Ipv4Net::new(a, 32).unwrap())),
                Ok(IpAddr::V6(a)) => Ok(Net::V6(Ipv6Net::new(a, 128).unwrap())),
                Err(_) => Err(net_value_error(text)),
            }
        }
    }
}

/// Contiguous netmask (e.g. `255.255.255.0`) → prefix length, or `None` if
/// the mask isn't a valid contiguous netmask.
fn netmask_to_prefix(mask: u32) -> Option<u8> {
    let ones = mask.leading_ones();
    // Valid iff the mask is `ones` leading 1-bits and nothing else.
    let expected = if ones == 0 {
        0
    } else if ones == 32 {
        u32::MAX
    } else {
        u32::MAX << (32 - ones)
    };
    if expected == mask {
        Some(ones as u8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
