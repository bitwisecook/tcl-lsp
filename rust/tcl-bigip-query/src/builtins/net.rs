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

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};

use crate::builtins::{BuiltinSpec, as_int, as_str, plain, type_name};
use crate::errors::QueryError;
use crate::value::Value;

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

// IPv4 classification (IANA range tables)

fn v4_in(ip: u32, net_addr: [u8; 4], prefix: u8) -> bool {
    let base = u32::from_be_bytes(net_addr);
    if prefix == 0 {
        return true;
    }
    let mask = u32::MAX << (32 - u32::from(prefix));
    (ip & mask) == (base & mask)
}

const V4_PRIVATE: &[([u8; 4], u8)] = &[
    ([0, 0, 0, 0], 8),
    ([10, 0, 0, 0], 8),
    ([127, 0, 0, 0], 8),
    ([169, 254, 0, 0], 16),
    ([172, 16, 0, 0], 12),
    ([192, 0, 0, 0], 24),
    ([192, 0, 0, 170], 31),
    ([192, 0, 2, 0], 24),
    ([192, 168, 0, 0], 16),
    ([198, 18, 0, 0], 15),
    ([198, 51, 100, 0], 24),
    ([203, 0, 113, 0], 24),
    ([240, 0, 0, 0], 4),
    ([255, 255, 255, 255], 32),
];

const V4_PRIVATE_EXCEPTIONS: &[[u8; 4]] = &[[192, 0, 0, 9], [192, 0, 0, 10]];

fn v4_is_private(ip: u32) -> bool {
    let in_private = V4_PRIVATE.iter().any(|(a, p)| v4_in(ip, *a, *p));
    let in_exception = V4_PRIVATE_EXCEPTIONS
        .iter()
        .any(|a| ip == u32::from_be_bytes(*a));
    in_private && !in_exception
}

fn v4_is_global(ip: u32) -> bool {
    // `not in 100.64.0.0/10` and `not is_private`.
    !v4_in(ip, [100, 64, 0, 0], 10) && !v4_is_private(ip)
}

fn v4_is_reserved(ip: u32) -> bool {
    v4_in(ip, [240, 0, 0, 0], 4)
}

fn v4_is_loopback(ip: u32) -> bool {
    v4_in(ip, [127, 0, 0, 0], 8)
}

fn v4_is_link_local(ip: u32) -> bool {
    v4_in(ip, [169, 254, 0, 0], 16)
}

fn v4_is_multicast(ip: u32) -> bool {
    v4_in(ip, [224, 0, 0, 0], 4)
}

fn v4_is_unspecified(ip: u32) -> bool {
    ip == 0
}

// IPv6 classification (IANA range tables)

fn v6_in(ip: u128, net: &str) -> bool {
    let n: Ipv6Net = net.parse().unwrap();
    let base = u128::from(n.network());
    let prefix = n.prefix_len();
    if prefix == 0 {
        return true;
    }
    let mask = u128::MAX << (128 - u32::from(prefix));
    (ip & mask) == (base & mask)
}

/// `IPv6Address.ipv4_mapped` — the embedded IPv4 if in `::ffff:0:0/96`.
fn v6_ipv4_mapped(ip: u128) -> Option<u32> {
    // `::ffff:0.0.0.0/96`: top 96 bits == 0x0000...0000ffff.
    if (ip >> 32) == 0xffff {
        Some((ip & 0xffff_ffff) as u32)
    } else {
        None
    }
}

const V6_PRIVATE: &[&str] = &[
    "::1/128",
    "::/128",
    "::ffff:0.0.0.0/96",
    "64:ff9b:1::/48",
    "100::/64",
    "2001::/23",
    "2001:db8::/32",
    "2002::/16",
    "3fff::/20",
    "fc00::/7",
    "fe80::/10",
];

const V6_PRIVATE_EXCEPTIONS: &[&str] = &[
    "2001:1::1/128",
    "2001:1::2/128",
    "2001:3::/32",
    "2001:4:112::/48",
    "2001:20::/28",
    "2001:30::/28",
];

const V6_RESERVED: &[&str] = &[
    "::/8", "100::/8", "200::/7", "400::/6", "800::/5", "1000::/4", "4000::/3", "6000::/3",
    "8000::/3", "a000::/3", "c000::/3", "e000::/4", "f000::/5", "f800::/6", "fe00::/9",
];

fn v6_is_private(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_private(m);
    }
    let in_private = V6_PRIVATE.iter().any(|n| v6_in(ip, n));
    let in_exception = V6_PRIVATE_EXCEPTIONS.iter().any(|n| v6_in(ip, n));
    in_private && !in_exception
}

fn v6_is_global(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_global(m);
    }
    !v6_is_private(ip)
}

fn v6_is_reserved(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_reserved(m);
    }
    V6_RESERVED.iter().any(|n| v6_in(ip, n))
}

fn v6_is_loopback(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_loopback(m);
    }
    ip == 1
}

fn v6_is_link_local(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_link_local(m);
    }
    v6_in(ip, "fe80::/10")
}

fn v6_is_multicast(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_multicast(m);
    }
    v6_in(ip, "ff00::/8")
}

fn v6_is_unspecified(ip: u128) -> bool {
    if let Some(m) = v6_ipv4_mapped(ip) {
        return v4_is_unspecified(m);
    }
    ip == 0
}

// per-address predicate dispatch

fn addr_is_ipv4(a: IpAddr) -> bool {
    a.is_ipv4()
}
fn addr_is_ipv6(a: IpAddr) -> bool {
    a.is_ipv6()
}

fn addr_is_private(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_private(u32::from(v)),
        IpAddr::V6(v) => v6_is_private(u128::from(v)),
    }
}
fn addr_is_loopback(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_loopback(u32::from(v)),
        IpAddr::V6(v) => v6_is_loopback(u128::from(v)),
    }
}
fn addr_is_unspecified(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_unspecified(u32::from(v)),
        IpAddr::V6(v) => v6_is_unspecified(u128::from(v)),
    }
}
fn addr_is_multicast(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_multicast(u32::from(v)),
        IpAddr::V6(v) => v6_is_multicast(u128::from(v)),
    }
}
fn addr_is_link_local(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_link_local(u32::from(v)),
        IpAddr::V6(v) => v6_is_link_local(u128::from(v)),
    }
}
fn addr_is_reserved(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_reserved(u32::from(v)),
        IpAddr::V6(v) => v6_is_reserved(u128::from(v)),
    }
}
fn addr_is_global(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_global(u32::from(v)),
        IpAddr::V6(v) => v6_is_global(u128::from(v)),
    }
}

/// `IPAddress.is_public` — stricter than `is_global` (excludes
/// multicast / link-local / loopback / unspecified / reserved).
fn addr_is_public(a: IpAddr) -> bool {
    addr_is_global(a)
        && !addr_is_multicast(a)
        && !addr_is_link_local(a)
        && !addr_is_loopback(a)
        && !addr_is_unspecified(a)
        && !addr_is_reserved(a)
}

/// `IPAddress.is_documentation` — the F5 typed layer checks canonical ranges.
fn addr_is_documentation(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => {
            let ip = u32::from(v);
            v4_in(ip, [192, 0, 2, 0], 24)
                || v4_in(ip, [198, 51, 100, 0], 24)
                || v4_in(ip, [203, 0, 113, 0], 24)
        }
        IpAddr::V6(v) => v6_in(u128::from(v), "2001:db8::/32"),
    }
}

// Typed value layer: IPAddress / FQDN, RouteDomain, Port, Partition,
// Folder, ObjectPath, Destination, Network, IPRange, PortSet.

const PART_VALID: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-.";

fn part_chars_ok(s: &str) -> bool {
    s.chars().all(|c| PART_VALID.contains(c))
}

/// An IP plus an optional `%rd` route domain.
#[derive(Clone)]
struct TypedIp {
    addr: IpAddr,
    route_domain: Option<u64>,
}

impl TypedIp {
    /// Parse `text` into a `TypedIp` (or `None`).
    fn try_parse(text: &str) -> Option<TypedIp> {
        let text = text.trim();
        let mut rd: Option<u64> = None;
        let mut body = text.to_string();
        if let Some(pos) = text.rfind('%') {
            let (host, rd_text) = (&text[..pos], &text[pos + 1..]);
            if !rd_text.is_empty() && rd_text.chars().all(|c| c.is_ascii_digit()) {
                rd = rd_text.parse::<u64>().ok();
                body = host.to_string();
            }
        }
        let addr = parse_ip_address(&body)?;
        Some(TypedIp {
            addr,
            route_domain: rd,
        })
    }

    fn to_string_canonical(&self) -> String {
        match self.route_domain {
            None => self.addr.to_string(),
            Some(rd) => format!("{}%{}", self.addr, rd),
        }
    }
}

/// Return whether `text` is a valid FQDN.
fn is_valid_fqdn(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.starts_with('.') || text.ends_with('.') {
        return false;
    }
    let labels: Vec<&str> = text.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }
    // Refuse an all-numeric 4-label value (looks like IPv4).
    if labels.len() == 4 && labels.iter().all(|l| l.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }
    true
}

/// `Address` sum type result.
enum TypedAddress {
    Ip(TypedIp),
    Fqdn(String),
}

impl TypedAddress {
    fn to_string_canonical(&self) -> String {
        match self {
            TypedAddress::Ip(ip) => ip.to_string_canonical(),
            TypedAddress::Fqdn(s) => s.clone(),
        }
    }
}

/// Parse an address: IP first, FQDN fallback.
fn parse_address(text: &str) -> Option<TypedAddress> {
    let text = text.trim();
    if let Some(ip) = TypedIp::try_parse(text) {
        return Some(TypedAddress::Ip(ip));
    }
    if is_valid_fqdn(text) {
        Some(TypedAddress::Fqdn(text.to_string()))
    } else {
        None
    }
}

/// A parsed port: numeric value plus its original spelling.
#[derive(Clone)]
struct TypedPort {
    port: u16,
    spelling: String,
}

impl TypedPort {
    fn try_parse(text: &str) -> Option<TypedPort> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let lower = text.to_lowercase();
        if lower == "any" || lower == "*" || lower == "0" {
            return Some(TypedPort {
                port: 0,
                spelling: text.to_string(),
            });
        }
        let value: i64 = text.parse().ok()?;
        if !(0..=65535).contains(&value) {
            return None;
        }
        Some(TypedPort {
            port: value as u16,
            spelling: text.to_string(),
        })
    }

    fn is_any(&self) -> bool {
        self.port == 0
    }

    fn render(&self) -> String {
        if !self.spelling.is_empty() {
            self.spelling.clone()
        } else if self.port == 0 {
            "any".to_string()
        } else {
            self.port.to_string()
        }
    }
}

/// A BIG-IP partition (canonical name with leading slash).
#[derive(Clone, PartialEq, Eq)]
struct Partition {
    name: String, // canonical with leading slash
}

impl Partition {
    fn try_parse(text: &str) -> Option<Partition> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let bare = text.trim_start_matches('/');
        if bare.is_empty() || bare.contains('/') || !part_chars_ok(bare) {
            return None;
        }
        Some(Partition {
            name: format!("/{bare}"),
        })
    }

    fn is_common(&self) -> bool {
        self.name.trim_start_matches('/') == "Common"
    }

    fn can_see(&self, other: &Partition) -> bool {
        self == other || other.is_common()
    }
}

/// A folder: a partition plus its nested path segments.
#[derive(Clone, PartialEq, Eq)]
struct Folder {
    partition: Partition,
    segments: Vec<String>,
}

impl Folder {
    fn try_parse(text: &str) -> Option<Folder> {
        let text = text.trim();
        if text.is_empty() || !text.starts_with('/') {
            return None;
        }
        let parts: Vec<&str> = text
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        let partition = Partition::try_parse(parts[0])?;
        let segments: Vec<String> = parts[1..].iter().map(|s| (*s).to_string()).collect();
        for seg in &segments {
            if !part_chars_ok(seg) {
                return None;
            }
        }
        Some(Folder {
            partition,
            segments,
        })
    }

    fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    fn render(&self) -> String {
        if self.is_root() {
            self.partition.name.clone()
        } else {
            format!("{}/{}", self.partition.name, self.segments.join("/"))
        }
    }
}

/// An object path: a folder plus the object's name.
struct ObjectPath {
    folder: Folder,
    name: String,
}

impl ObjectPath {
    fn try_parse(text: &str) -> Option<ObjectPath> {
        let text = text.trim();
        if text.is_empty() || !text.starts_with('/') {
            return None;
        }
        let parts: Vec<&str> = text
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() < 2 {
            return None;
        }
        let leaf = parts[parts.len() - 1];
        if !part_chars_ok(leaf) {
            return None;
        }
        let folder_text = format!("/{}", parts[..parts.len() - 1].join("/"));
        let folder = Folder::try_parse(&folder_text)?;
        Some(ObjectPath {
            folder,
            name: leaf.to_string(),
        })
    }

    fn partition(&self) -> &Partition {
        &self.folder.partition
    }

    fn render(&self) -> String {
        if self.folder.is_root() {
            format!("{}/{}", self.folder.partition.name, self.name)
        } else {
            format!("{}/{}", self.folder.render(), self.name)
        }
    }
}

/// A parsed BIG-IP destination (address, port, folder, route domain).
struct Destination {
    address: TypedAddress,
    port: TypedPort,
    folder: Option<Folder>,
    route_domain: Option<u64>,
    ipv6_brackets: bool,
    port_separator: char,
}

/// Strip a trailing `%N` route domain.
/// Returns `(addr-without-rd, route-domain)`.
fn split_route_domain(text: &str) -> (String, Option<u64>) {
    if !text.contains('%') {
        return (text.to_string(), None);
    }
    let (addr, rd_text) = text.split_once('%').unwrap();
    let mut rd_digits = String::new();
    for ch in rd_text.chars() {
        if ch.is_ascii_digit() {
            rd_digits.push(ch);
        } else {
            break;
        }
    }
    let remainder = &rd_text[rd_digits.len()..];
    if rd_digits.is_empty() {
        return (text.to_string(), None);
    }
    // RouteDomain.parse: non-negative integer.
    let rd: u64 = rd_digits.parse().unwrap();
    (format!("{addr}{remainder}"), Some(rd))
}

/// True when a path segment looks like a host.
fn looks_like_address(segment: &str) -> bool {
    if segment.contains(':') {
        return true;
    }
    segment.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Split a leading folder prefix off `text`, returning `(folder, remainder)`.
fn split_folder_prefix(text: &str) -> (Option<Folder>, String) {
    if !text.starts_with('/') {
        return (None, text.to_string());
    }
    let parts: Vec<&str> = text.trim_start_matches('/').split('/').collect();
    if parts.len() < 2 {
        return (None, text.to_string());
    }
    let mut folder_segments: Vec<&str> = vec![parts[0]];
    let mut host_index = None;
    for (i, seg) in parts.iter().enumerate().skip(1) {
        if seg.starts_with('[') || looks_like_address(seg) {
            host_index = Some(i);
            break;
        }
        folder_segments.push(seg);
    }
    let host_index = if let Some(i) = host_index {
        i
    } else {
        // Every segment folder-shaped — last is the host.
        let hi = parts.len() - 1;
        folder_segments = parts[..hi].to_vec();
        hi
    };
    let folder_text = format!("/{}", folder_segments.join("/"));
    match Folder::try_parse(&folder_text) {
        None => (None, text.to_string()),
        Some(folder) => {
            let remainder = parts[host_index..].join("/");
            (Some(folder), remainder)
        }
    }
}

impl Destination {
    /// Parse a destination (returns `None` instead of raising). The
    /// length comes from handling the bracket / IPv6 / IPv4-FQDN branches
    /// individually.
    #[allow(clippy::too_many_lines)]
    fn try_parse(text: &str) -> Option<Destination> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (folder, rest) = split_folder_prefix(text);

        // Bracketed IPv6.
        if let Some(stripped) = rest.strip_prefix('[') {
            let close = stripped.find(']')?;
            let addr_text = &stripped[..close];
            let port_part = &stripped[close + 1..];
            let (addr_clean, route_domain) = split_route_domain(addr_text);
            let ip = TypedIp::try_parse(&addr_clean)?;
            if port_part.is_empty() {
                return Some(Destination {
                    address: TypedAddress::Ip(ip),
                    port: TypedPort {
                        port: 0,
                        spelling: String::new(),
                    },
                    folder,
                    route_domain,
                    ipv6_brackets: true,
                    port_separator: ':',
                });
            }
            let sep = port_part.chars().next().unwrap();
            if sep != ':' && sep != '.' {
                return None;
            }
            let port = TypedPort::try_parse(&port_part[1..])?;
            return Some(Destination {
                address: TypedAddress::Ip(ip),
                port,
                folder,
                route_domain,
                ipv6_brackets: true,
                port_separator: sep,
            });
        }

        // No brackets.
        let (addr_part, route_domain) = split_route_domain(&rest);
        if addr_part.matches(':').count() >= 2 {
            // Unbracketed IPv6 — `.` is the port separator.
            match addr_part.rfind('.') {
                None => {
                    let ip = TypedIp::try_parse(&addr_part)?;
                    Some(Destination {
                        address: TypedAddress::Ip(ip),
                        port: TypedPort {
                            port: 0,
                            spelling: String::new(),
                        },
                        folder,
                        route_domain,
                        ipv6_brackets: false,
                        port_separator: '.',
                    })
                }
                Some(sep) => {
                    let addr_text = &addr_part[..sep];
                    let port_text = &addr_part[sep + 1..];
                    // Prefer the `<ip>.<port>` split; but a portless IPv4-mapped
                    // address (`::ffff:10.1.1.1`) has dots *inside* the address,
                    // so the last `.` is not a port separator. Fall back to
                    // parsing the whole part as the address with no port when
                    // the split doesn't yield a valid IP + port (issue 194).
                    if let (Some(ip), Some(port)) = (
                        TypedIp::try_parse(addr_text),
                        TypedPort::try_parse(port_text),
                    ) {
                        Some(Destination {
                            address: TypedAddress::Ip(ip),
                            port,
                            folder,
                            route_domain,
                            ipv6_brackets: false,
                            port_separator: '.',
                        })
                    } else {
                        let ip = TypedIp::try_parse(&addr_part)?;
                        Some(Destination {
                            address: TypedAddress::Ip(ip),
                            port: TypedPort {
                                port: 0,
                                spelling: String::new(),
                            },
                            folder,
                            route_domain,
                            ipv6_brackets: false,
                            port_separator: '.',
                        })
                    }
                }
            }
        } else {
            // IPv4 / FQDN — port separator is the last `:`.
            match addr_part.rfind(':') {
                None => {
                    let address = parse_address(&addr_part)?;
                    Some(Destination {
                        address,
                        port: TypedPort {
                            port: 0,
                            spelling: String::new(),
                        },
                        folder,
                        route_domain,
                        ipv6_brackets: false,
                        port_separator: ':',
                    })
                }
                Some(sep) => {
                    let addr_text = &addr_part[..sep];
                    let port_text = &addr_part[sep + 1..];
                    let address = parse_address(addr_text)?;
                    let port = TypedPort::try_parse(port_text)?;
                    Some(Destination {
                        address,
                        port,
                        folder,
                        route_domain,
                        ipv6_brackets: false,
                        port_separator: ':',
                    })
                }
            }
        }
    }

    fn render(&self) -> String {
        let mut addr_text = self.address.to_string_canonical();
        let is_v6 = matches!(&self.address, TypedAddress::Ip(ip) if ip.addr.is_ipv6());
        if is_v6 && self.ipv6_brackets {
            addr_text = format!("[{addr_text}]");
        }
        // route_domain rendering (non-default).
        if let Some(rd) = self.route_domain
            && rd != 0
        {
            if self.ipv6_brackets {
                let mut chars: Vec<char> = addr_text.chars().collect();
                chars.pop(); // remove trailing ']'
                addr_text = format!("{}%{}]", chars.into_iter().collect::<String>(), rd);
            } else {
                addr_text = format!("{addr_text}%{rd}");
            }
        }
        let mut out = addr_text;
        if !(self.port.is_any() && self.port.spelling.is_empty()) {
            out = format!("{out}{}{}", self.port_separator, self.port.render());
        }
        if let Some(folder) = &self.folder {
            out = format!("{}/{out}", folder.render());
        }
        out
    }
}

/// Split a destination string into `(partition_prefix, address,
/// route_domain, port)`.
fn split_destination(value: &str) -> (String, String, String, String) {
    if let Some(dest) = Destination::try_parse(value) {
        let partition_prefix = match &dest.folder {
            Some(f) => format!("{}/", f.render()),
            None => String::new(),
        };
        let addr_text = dest.address.to_string_canonical();
        let rd_text = match dest.route_domain {
            Some(rd) if rd != 0 => rd.to_string(),
            _ => String::new(),
        };
        let port_text = if dest.port.is_any() && dest.port.spelling.is_empty() {
            String::new()
        } else if dest.port.is_any() {
            dest.port.spelling.clone()
        } else {
            dest.port.port.to_string()
        };
        return (partition_prefix, addr_text, rd_text, port_text);
    }
    // Legacy regex fallback: ^(/[^/]+/)?([^%:/\s]+)(%[A-Za-z0-9_-]+)?(:\d+)?$
    legacy_dest_re(value).unwrap_or_else(|| {
        (
            String::new(),
            value.to_string(),
            String::new(),
            String::new(),
        )
    })
}

/// Fallback regex match for a destination string.
fn legacy_dest_re(value: &str) -> Option<(String, String, String, String)> {
    let mut rest = value;
    let mut partition = String::new();
    // partition: (/[^/]+/)?
    if let Some(after_first) = rest.strip_prefix('/')
        && let Some(slash2) = after_first.find('/')
    {
        let mid = &after_first[..slash2];
        if !mid.is_empty() && !mid.contains('/') {
            partition = format!("/{mid}/");
            rest = &after_first[slash2 + 1..];
        }
    }
    // addr: [^%:/\s]+
    let addr_end = rest
        .find(|c: char| c == '%' || c == ':' || c == '/' || c.is_whitespace())
        .unwrap_or(rest.len());
    let addr = &rest[..addr_end];
    if addr.is_empty() {
        return None;
    }
    let mut tail = &rest[addr_end..];
    // rd: (%[A-Za-z0-9_-]+)?
    let mut rd = String::new();
    if let Some(after_pct) = tail.strip_prefix('%') {
        let rd_end = after_pct
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after_pct.len());
        if rd_end == 0 {
            return None;
        }
        rd = after_pct[..rd_end].to_string();
        tail = &after_pct[rd_end..];
    }
    // port: (:\d+)?
    let mut port = String::new();
    if let Some(after_colon) = tail.strip_prefix(':') {
        if after_colon.is_empty() || !after_colon.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        port = after_colon.to_string();
        tail = "";
    }
    if !tail.is_empty() {
        return None;
    }
    Some((partition, addr.to_string(), rd, port))
}

/// Reassemble a destination string from its parts.
fn rebuild_destination(partition: &str, address: &str, route_domain: &str, port: &str) -> String {
    let mut out = format!("{partition}{address}");
    if !route_domain.is_empty() {
        out = format!("{out}%{route_domain}");
    }
    if !port.is_empty() {
        out = format!("{out}:{port}");
    }
    out
}

/// Coerce to a typed `Address`.
fn typed_address(value: &Value, name: &str) -> Result<Option<TypedAddress>, QueryError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let s = as_str(value, name, 1)?;
    if s.is_empty() {
        return Ok(None);
    }
    if let Some(dest) = Destination::try_parse(&s) {
        return Ok(Some(dest.address));
    }
    Ok(parse_address(&s))
}

/// Coerce to a typed `Network`.
fn typed_network(value: &Value, name: &str, arg: usize) -> Result<Option<Net>, QueryError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let s = as_str(value, name, arg)?;
    if s.is_empty() {
        return Ok(None);
    }
    Ok(network_try_parse(&s))
}

/// Parse a network: integer CIDR, dotted-quad netmask,
/// space-separated `ADDR MASK`, and the `default` keywords.
fn network_try_parse(text: &str) -> Option<Net> {
    let original = text.trim();
    let mut t = original.to_string();
    if t == "default" {
        return parse_network("0.0.0.0/0").ok();
    }
    if t == "default-inet6" {
        return parse_network("::/0").ok();
    }
    if t.contains(' ') && !t.contains('/') {
        let (addr_part, mask_part) = t.split_once(' ').unwrap();
        t = format!("{}/{}", addr_part.trim(), mask_part.trim());
    }
    parse_network(&t).ok()
}

// Builtins

fn bi_ip(args: &[Value]) -> Result<Value, QueryError> {
    if args.len() == 1 {
        let s = as_str(&args[0], "ip", 1)?;
        let (_, addr, _, _) = split_destination(&s);
        if parse_ip_address(&addr).is_none() {
            return Err(QueryError::builtin(format!(
                "ip: invalid address {}: {}",
                py_str_repr(&addr),
                addr_value_error(&addr)
            )));
        }
        return Ok(Value::Str(addr));
    }
    let net_str = as_str(&args[0], "ip", 1)?;
    let src_str = as_str(&args[1], "ip", 2)?;
    let network = parse_network(&net_str).map_err(|_| {
        QueryError::builtin(format!(
            "ip: invalid network {}: {}",
            py_str_repr(&net_str),
            net_value_error(&net_str)
        ))
    })?;
    let (partition, src_addr, rd, port) = split_destination(&src_str);
    let src_ip = parse_ip_address(&src_addr).ok_or_else(|| {
        QueryError::builtin(format!(
            "ip: invalid source address {}: {}",
            py_str_repr(&src_addr),
            addr_value_error(&src_addr)
        ))
    })?;
    match (network, src_ip) {
        (Net::V4(_), IpAddr::V6(_)) => {
            return Err(QueryError::builtin(
                "ip: cannot rebase an IPv6 address into an IPv4 network",
            ));
        }
        (Net::V6(_), IpAddr::V4(_)) => {
            return Err(QueryError::builtin(
                "ip: cannot rebase an IPv4 address into an IPv6 network",
            ));
        }
        _ => {}
    }
    let new_addr = match (network, src_ip) {
        (Net::V4(n), IpAddr::V4(s)) => {
            let netmask = u32::MAX
                .checked_shl(32 - u32::from(n.prefix_len()))
                .unwrap_or(0);
            let host_bits = u32::from(s) & !netmask;
            let new_int = u32::from(n.network()) | host_bits;
            Ipv4Addr::from(new_int).to_string()
        }
        (Net::V6(n), IpAddr::V6(s)) => {
            let netmask: u128 = if n.prefix_len() == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(n.prefix_len()))
            };
            let host_bits = u128::from(s) & !netmask;
            let new_int = u128::from(n.network()) | host_bits;
            Ipv6Addr::from(new_int).to_string()
        }
        _ => unreachable!("family mismatch handled above"),
    };
    Ok(Value::Str(rebuild_destination(
        &partition, &new_addr, &rd, &port,
    )))
}

fn bi_ip_translate(args: &[Value]) -> Result<Value, QueryError> {
    let src_str = as_str(&args[0], "ip_translate", 1)?;
    let dst_str = as_str(&args[1], "ip_translate", 2)?;
    let addr_str = as_str(&args[2], "ip_translate", 3)?;
    let src_net = parse_network(&src_str).map_err(|_| {
        QueryError::builtin(format!(
            "ip_translate: invalid src_net {}: {}",
            py_str_repr(&src_str),
            net_value_error(&src_str)
        ))
    })?;
    let dst_net = parse_network(&dst_str).map_err(|_| {
        QueryError::builtin(format!(
            "ip_translate: invalid dst_net {}: {}",
            py_str_repr(&dst_str),
            net_value_error(&dst_str)
        ))
    })?;
    let (_, bare_addr, _, _) = split_destination(&addr_str);
    let src_ip = parse_ip_address(&bare_addr).ok_or_else(|| {
        QueryError::builtin(format!(
            "ip_translate: invalid addr {}: {}",
            py_str_repr(&bare_addr),
            addr_value_error(&bare_addr)
        ))
    })?;
    let src_ip_int = ip_to_u128(src_ip);
    // `src_ip not in src_net`.
    if !net_contains_addr(src_net, src_ip) {
        return Err(QueryError::builtin(format!(
            "ip_translate: {} is not within src_net {}",
            py_str_repr(&bare_addr),
            py_str_repr(&src_str)
        )));
    }
    let src_host_bits = src_net.max_prefixlen() - src_net.prefix_len();
    let dst_host_bits = dst_net.max_prefixlen() - dst_net.prefix_len();
    if src_host_bits > dst_host_bits {
        return Err(QueryError::builtin(format!(
            "ip_translate: dst_net {} is too specific to hold the host bits of src_net {} ({}-bit host vs {}-bit host)",
            py_str_repr(&dst_str),
            py_str_repr(&src_str),
            src_host_bits,
            dst_host_bits
        )));
    }
    let host_offset = src_ip_int - src_net.network_int();
    let new_int = dst_net.network_int() | host_offset;
    let result = match dst_net {
        Net::V4(_) => Ipv4Addr::from(new_int as u32).to_string(),
        Net::V6(_) => Ipv6Addr::from(new_int).to_string(),
    };
    Ok(Value::Str(result))
}

fn ip_to_u128(a: IpAddr) -> u128 {
    match a {
        IpAddr::V4(v) => u128::from(u32::from(v)),
        IpAddr::V6(v) => u128::from(v),
    }
}

fn net_contains_addr(n: Net, a: IpAddr) -> bool {
    match (n, a) {
        (Net::V4(net), IpAddr::V4(ip)) => net.contains(&ip),
        (Net::V6(net), IpAddr::V6(ip)) => net.contains(&ip),
        _ => false,
    }
}

fn bi_net(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "net", 1)?;
    let network = parse_network(&s).map_err(|_| {
        QueryError::builtin(format!(
            "net: invalid value {}: {}",
            py_str_repr(&s),
            net_value_error(&s)
        ))
    })?;
    Ok(Value::Str(network.network_str()))
}

fn bi_host(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "host", 1)?;
    let (_, addr, _, _) = split_destination(&s);
    Ok(Value::Str(addr))
}

fn bi_port(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port", 1)?;
    let (_, _, _, port) = split_destination(&s);
    if port.is_empty() {
        Ok(Value::Null)
    } else {
        // `int(port)`; a non-numeric port (e.g. the wildcard spelling
        // `"any"`) raises the ValueError verbatim.
        match port.parse::<i64>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => Err(QueryError::builtin(format!(
                "invalid literal for int() with base 10: {}",
                py_str_repr(&port)
            ))),
        }
    }
}

fn bi_in_cidr(args: &[Value]) -> Result<Value, QueryError> {
    let addr_s = as_str(&args[0], "in_cidr", 1)?;
    let net_s = as_str(&args[1], "in_cidr", 2)?;
    let (_, host, _, _) = split_destination(&addr_s);
    let Some(ip) = parse_ip_address(&host) else {
        return Ok(Value::Bool(false));
    };
    let net = parse_network(&net_s).map_err(|_| {
        QueryError::builtin(format!(
            "in_cidr: invalid network {}: {}",
            py_str_repr(&net_s),
            net_value_error(&net_s)
        ))
    })?;
    Ok(Value::Bool(net_contains_addr(net, ip)))
}

fn bi_route_domain(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "route_domain", 1)?;
    let (_, _, rd, _) = split_destination(&s);
    if rd.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Str(rd))
    }
}

fn bi_with_route_domain(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_route_domain", 1)?;
    let (partition, addr, _, port) = split_destination(&s);
    let new_rd = match &args[1] {
        Value::Null => String::new(),
        Value::Str(rd) if rd.is_empty() => String::new(),
        Value::Bool(_) => {
            return Err(QueryError::builtin(
                "with_route_domain: rd cannot be a boolean",
            ));
        }
        Value::Int(i) => i.to_string(),
        Value::Str(rd) => rd.clone(),
        Value::PathRef(p) => p.full_path.clone(),
        other => {
            return Err(QueryError::builtin(format!(
                "with_route_domain: rd must be a string, integer, or null, got {}",
                type_name(other)
            )));
        }
    };
    Ok(Value::Str(rebuild_destination(
        &partition, &addr, &new_rd, &port,
    )))
}

// predicates

fn addr_predicate(value: &Value, name: &str, f: fn(IpAddr) -> bool) -> Result<Value, QueryError> {
    let a = typed_address(value, name)?;
    Ok(Value::Bool(match a {
        Some(TypedAddress::Ip(ip)) => f(ip.addr),
        _ => false,
    }))
}

fn bi_is_ipv4(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_ipv4", addr_is_ipv4)
}
fn bi_is_ipv6(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_ipv6", addr_is_ipv6)
}
fn bi_is_fqdn(args: &[Value]) -> Result<Value, QueryError> {
    let a = typed_address(&args[0], "is_fqdn")?;
    Ok(Value::Bool(matches!(a, Some(TypedAddress::Fqdn(_)))))
}
fn bi_is_private(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_private", addr_is_private)
}
fn bi_is_loopback(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_loopback", addr_is_loopback)
}
fn bi_is_unspecified(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_unspecified", addr_is_unspecified)
}
fn bi_is_multicast(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_multicast", addr_is_multicast)
}
fn bi_is_link_local(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_link_local", addr_is_link_local)
}
fn bi_is_reserved(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_reserved", addr_is_reserved)
}
fn bi_is_public(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_public", addr_is_public)
}
fn bi_is_documentation(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_documentation", addr_is_documentation)
}

fn bi_is_wildcard_port(args: &[Value]) -> Result<Value, QueryError> {
    if matches!(&args[0], Value::Null) {
        return Ok(Value::Bool(false));
    }
    let s = as_str(&args[0], "is_wildcard_port", 1)?;
    let result = Destination::try_parse(&s).is_some_and(|d| d.port.is_any());
    Ok(Value::Bool(result))
}

// network info

fn bi_prefix_length(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "prefix_length", 1)? {
        None => Ok(Value::Null),
        Some(n) => Ok(Value::Int(i64::from(n.prefix_len()))),
    }
}

fn bi_network_address(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "network_address", 1)? {
        None => Ok(Value::Null),
        Some(n) => Ok(Value::Str(n.network_address_str())),
    }
}

fn bi_broadcast_address(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "broadcast_address", 1)? {
        None => Ok(Value::Null),
        Some(n) => Ok(Value::Str(n.broadcast_address_str())),
    }
}

/// First / last usable host of the network, then the `first_host` /
/// `last_host` fallback to the network address when the host range is empty.
///
/// `hosts()` excludes the network + broadcast addresses for IPv4 prefixes
/// shorter than /31, and the network address for IPv6 prefixes shorter than
/// /127. For /31, /32 (v6 /127, /128) it yields the whole range. The
/// builtins fall back to the network address when the list is empty — which,
/// given these ranges, never happens, so first/last always exist.
fn net_first_last_host(n: Net) -> (String, String) {
    let lo = n.network_int();
    let hi = n.broadcast_int();
    match n {
        Net::V4(_) => {
            let (first, last) = if n.prefix_len() >= 31 {
                (lo, hi)
            } else {
                (lo + 1, hi - 1)
            };
            (
                Ipv4Addr::from(first as u32).to_string(),
                Ipv4Addr::from(last as u32).to_string(),
            )
        }
        Net::V6(_) => {
            let (first, last) = if n.prefix_len() >= 127 {
                (lo, hi)
            } else {
                (lo + 1, hi)
            };
            (
                Ipv6Addr::from(first).to_string(),
                Ipv6Addr::from(last).to_string(),
            )
        }
    }
}

fn bi_first_host(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "first_host", 1)? {
        None => Ok(Value::Null),
        Some(n) => {
            let (first, _) = net_first_last_host(n);
            Ok(Value::Str(first))
        }
    }
}

fn bi_last_host(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "last_host", 1)? {
        None => Ok(Value::Null),
        Some(n) => {
            let (_, last) = net_first_last_host(n);
            Ok(Value::Str(last))
        }
    }
}

/// Represent a u128 address count as a query number: an exact `Int` when it
/// fits i64, otherwise a `Float` (approximate but correctly signed and scaled).
/// A raw `as i64` cast truncated 2^96-scale IPv6 counts to negative garbage.
fn count_value(count: u128) -> Value {
    i64::try_from(count).map_or_else(|_| Value::Float(count as f64), Value::Int)
}

fn bi_host_count(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "host_count", 1)? {
        None => Ok(Value::Null),
        Some(n) => {
            let total = n.num_addresses();
            let count = if total <= 2 { total } else { total - 2 };
            Ok(count_value(count))
        }
    }
}

// CIDR algebra

fn as_sequence(v: &Value, name: &str, arg: usize) -> Result<Vec<Value>, QueryError> {
    crate::builtins::as_sequence(v, name, arg)
}

fn bi_collapse_cidrs(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "collapse_cidrs", 1)?;
    let mut v4: Vec<Ipv4Net> = Vec::new();
    let mut v6: Vec<Ipv6Net> = Vec::new();
    for item in &items {
        if let Some(net) = typed_network(item, "collapse_cidrs", 1)? {
            match net {
                Net::V4(n) => v4.push(n),
                Net::V6(n) => v6.push(n),
            }
        }
    }
    let mut out: Vec<Value> = collapse_v4(&v4)
        .into_iter()
        .map(|n| Value::Str(format!("{}/{}", n.network(), n.prefix_len())))
        .collect();
    out.extend(
        collapse_v6(&v6)
            .into_iter()
            .map(|n| Value::Str(format!("{}/{}", n.network(), n.prefix_len()))),
    );
    Ok(Value::List(out))
}

/// Collapse adjacent / overlapping IPv4 networks.
fn collapse_v4(nets: &[Ipv4Net]) -> Vec<Ipv4Net> {
    let ranges: Vec<(u128, u128)> = nets
        .iter()
        .map(|n| {
            (
                u128::from(u32::from(n.network())),
                u128::from(u32::from(n.broadcast())),
            )
        })
        .collect();
    collapse_ranges(&ranges, 32)
        .into_iter()
        .map(|(addr, prefix)| Ipv4Net::new(Ipv4Addr::from(addr as u32), prefix).unwrap())
        .collect()
}

fn collapse_v6(nets: &[Ipv6Net]) -> Vec<Ipv6Net> {
    let ranges: Vec<(u128, u128)> = nets
        .iter()
        .map(|n| (u128::from(n.network()), u128::from(n.broadcast())))
        .collect();
    collapse_ranges(&ranges, 128)
        .into_iter()
        .map(|(addr, prefix)| Ipv6Net::new(Ipv6Addr::from(addr), prefix).unwrap())
        .collect()
}

/// Collapse a set of `[first, last]` ranges into the minimal sorted list of
/// CIDR `(network_int, prefix)` pairs.
fn collapse_ranges(ranges: &[(u128, u128)], max_prefix: u8) -> Vec<(u128, u8)> {
    if ranges.is_empty() {
        return Vec::new();
    }
    // Merge overlapping / adjacent ranges (sorted by start).
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable();
    let mut merged: Vec<(u128, u128)> = Vec::new();
    for (lo, hi) in sorted {
        if let Some(last) = merged.last_mut() {
            // Adjacent if lo == last.1 + 1 (guard u128 overflow).
            if lo <= last.1 || (last.1 != u128::MAX && lo == last.1 + 1) {
                if hi > last.1 {
                    last.1 = hi;
                }
                continue;
            }
        }
        merged.push((lo, hi));
    }
    // Summarise each merged range into CIDRs.
    let mut out = Vec::new();
    for (lo, hi) in merged {
        summarize_range(lo, hi, max_prefix, &mut out);
    }
    out
}

/// Summarise the address range `[first, last]` as
/// `(network_int, prefix)` pairs, appended to `out`.
fn summarize_range(first: u128, last: u128, max_prefix: u8, out: &mut Vec<(u128, u8)>) {
    let mut first = first;
    while first <= last {
        // Largest block aligned at `first` that doesn't exceed `last`.
        let trailing = if first == 0 {
            u32::from(max_prefix)
        } else {
            first.trailing_zeros()
        }
        .min(u32::from(max_prefix));
        // Block size limited by remaining count (last - first + 1).
        let remaining = last - first; // last >= first
        let max_bits_by_count = bit_length(remaining + 1) - 1;
        let nbits = trailing.min(max_bits_by_count);
        let prefix = max_prefix - nbits as u8;
        out.push((first, prefix));
        let block = 1u128 << nbits;
        if first.checked_add(block).is_none() {
            break;
        }
        first += block;
        if first == 0 {
            break;
        }
    }
}

/// Bit length of a u128 (number of bits to represent it); `bit_length(0)==0`.
fn bit_length(n: u128) -> u32 {
    128 - n.leading_zeros()
}

fn bi_supernet_of(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "supernet_of", 1)?;
    let mut nets: Vec<Net> = Vec::new();
    for item in &items {
        if let Some(net) = typed_network(item, "supernet_of", 1)? {
            nets.push(net);
            continue;
        }
        if let Some(TypedAddress::Ip(ip)) = typed_address(item, "supernet_of")? {
            let prefix = if ip.addr.is_ipv4() { 32 } else { 128 };
            let cidr = format!("{}/{}", ip.addr, prefix);
            if let Ok(net) = parse_network(&cidr) {
                nets.push(net);
            }
        }
    }
    if nets.is_empty() {
        return Ok(Value::Null);
    }
    let any_v4 = nets.iter().any(|n| n.is_v4());
    let any_v6 = nets.iter().any(|n| !n.is_v4());
    if any_v4 && any_v6 {
        return Err(QueryError::builtin(
            "supernet_of: cannot mix IPv4 and IPv6 inputs",
        ));
    }
    let is_v4 = any_v4;
    let max_prefix: u8 = if is_v4 { 32 } else { 128 };
    let lowest = nets.iter().map(|n| n.network_int()).min().unwrap();
    let highest = nets.iter().map(|n| n.broadcast_int()).max().unwrap();
    let mut prefix = nets.iter().map(|n| n.prefix_len()).min().unwrap();
    loop {
        // candidate = the network for (lowest, prefix), host bits masked off
        let (cand_net, cand_bcast) = network_at(lowest, prefix, max_prefix);
        if cand_net <= lowest && cand_bcast >= highest {
            let addr = format_ip_int(cand_net, is_v4);
            return Ok(Value::Str(format!("{addr}/{prefix}")));
        }
        if prefix == 0 {
            break;
        }
        prefix -= 1;
    }
    Ok(Value::Str(if is_v4 {
        "0.0.0.0/0".to_string()
    } else {
        "::/0".to_string()
    }))
}

/// `(network_addr, broadcast_addr)` ints for the network containing `addr`
/// with the given `prefix` (host bits masked off).
fn network_at(addr: u128, prefix: u8, max_prefix: u8) -> (u128, u128) {
    let host_bits = u32::from(max_prefix - prefix);
    let mask = if host_bits >= 128 {
        0
    } else {
        u128::MAX << host_bits
    };
    // Apply within the family's bit-width.
    let family_mask = if max_prefix == 32 {
        u128::from(u32::MAX)
    } else {
        u128::MAX
    };
    let net = addr & mask & family_mask;
    let bcast = net | (!mask & family_mask);
    (net, bcast)
}

fn format_ip_int(v: u128, is_v4: bool) -> String {
    if is_v4 {
        Ipv4Addr::from(v as u32).to_string()
    } else {
        Ipv6Addr::from(v).to_string()
    }
}

fn bi_subnet_of(args: &[Value]) -> Result<Value, QueryError> {
    let sub = typed_network(&args[0], "subnet_of", 1)?;
    let sup = typed_network(&args[1], "subnet_of", 2)?;
    let (Some(sub), Some(sup)) = (sub, sup) else {
        return Ok(Value::Bool(false));
    };
    if sub.is_v4() != sup.is_v4() {
        return Ok(Value::Bool(false));
    }
    // subnet_of: sub network range entirely within sup, and sub prefix >= sup prefix.
    let result = sub.prefix_len() >= sup.prefix_len()
        && sup.network_int() <= sub.network_int()
        && sup.broadcast_int() >= sub.broadcast_int();
    Ok(Value::Bool(result))
}

fn bi_overlaps(args: &[Value]) -> Result<Value, QueryError> {
    let a = typed_network(&args[0], "overlaps", 1)?;
    let b = typed_network(&args[1], "overlaps", 2)?;
    let (Some(a), Some(b)) = (a, b) else {
        return Ok(Value::Bool(false));
    };
    if a.is_v4() != b.is_v4() {
        return Ok(Value::Bool(false));
    }
    let result = a.network_int() <= b.broadcast_int() && b.network_int() <= a.broadcast_int();
    Ok(Value::Bool(result))
}

// destination edit

fn bi_with_port(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_port", 1)?;
    let parsed = Destination::try_parse(&s).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_port: cannot parse destination {}",
            py_str_repr(&s)
        ))
    })?;
    let new_port = match &args[1] {
        Value::Null => TypedPort {
            port: 0,
            spelling: String::new(),
        },
        Value::Str(p) if p.is_empty() => TypedPort {
            port: 0,
            spelling: String::new(),
        },
        Value::Bool(_) => {
            return Err(QueryError::builtin("with_port: port cannot be a boolean"));
        }
        Value::Int(i) => {
            if !(0..=65535).contains(i) {
                return Err(QueryError::builtin(format!(
                    "with_port: port out of range ({i})"
                )));
            }
            TypedPort {
                port: *i as u16,
                spelling: i.to_string(),
            }
        }
        Value::Str(p) => TypedPort::try_parse(p).ok_or_else(|| {
            // Port.parse raised ValueError — surfaced as the raw message.
            port_parse_error(p)
        })?,
        Value::PathRef(p) => {
            TypedPort::try_parse(&p.full_path).ok_or_else(|| port_parse_error(&p.full_path))?
        }
        other => {
            return Err(QueryError::builtin(format!(
                "with_port: port must be a string, integer, or null, got {}",
                type_name(other)
            )));
        }
    };
    let dest = Destination {
        address: parsed.address,
        port: new_port,
        folder: parsed.folder,
        route_domain: parsed.route_domain,
        ipv6_brackets: parsed.ipv6_brackets,
        port_separator: parsed.port_separator,
    };
    Ok(Value::Str(dest.render()))
}

/// `Port.parse` `ValueError` text, surfaced verbatim when `with_port` is given
/// an unparseable port string.
fn port_parse_error(text: &str) -> QueryError {
    let t = text.trim();
    if t.is_empty() {
        return QueryError::builtin("Port: empty input");
    }
    let lower = t.to_lowercase();
    if lower == "any" || lower == "*" || lower == "0" {
        // would have parsed
        return QueryError::builtin("Port: empty input");
    }
    match t.parse::<i64>() {
        Err(_) => QueryError::builtin(format!("Port: not a number ({})", py_str_repr(t))),
        Ok(v) => QueryError::builtin(format!("Port: out of range ({v})")),
    }
}

fn bi_with_host(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_host", 1)?;
    let h = as_str(&args[1], "with_host", 2)?;
    let parsed = Destination::try_parse(&s).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_host: cannot parse destination {}",
            py_str_repr(&s)
        ))
    })?;
    let new_addr = parse_address(&h).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_host: cannot parse host {}: {}",
            py_str_repr(&h),
            // parse_address raises FQDN's ValueError; the typical failure is a
            // value that's neither IP nor FQDN. The FQDN error text varies;
            // reproduce the most common shape.
            fqdn_value_error(&h)
        ))
    })?;
    let new_is_v6 = matches!(&new_addr, TypedAddress::Ip(ip) if ip.addr.is_ipv6());
    let (new_brackets, new_separator) = if new_is_v6 {
        (parsed.ipv6_brackets, parsed.port_separator)
    } else {
        (false, ':')
    };
    let dest = Destination {
        address: new_addr,
        port: parsed.port,
        folder: parsed.folder,
        route_domain: parsed.route_domain,
        ipv6_brackets: new_brackets,
        port_separator: new_separator,
    };
    Ok(Value::Str(dest.render()))
}

/// FQDN parse `ValueError` text (best-effort).
fn fqdn_value_error(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return "FQDN: empty input".to_string();
    }
    if t.starts_with('.') || t.ends_with('.') {
        return format!("FQDN: leading/trailing dot in {}", py_str_repr(t));
    }
    let labels: Vec<&str> = t.split('.').collect();
    if labels.len() < 2 {
        return format!("FQDN: must have at least one dot ({})", py_str_repr(t));
    }
    for label in &labels {
        if label.is_empty() {
            return format!("FQDN: empty label in {}", py_str_repr(t));
        }
        if label.len() > 63 {
            return format!("FQDN: label longer than 63 chars in {}", py_str_repr(t));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return format!("FQDN: label leading/trailing hyphen in {}", py_str_repr(t));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return format!("FQDN: invalid char in label of {}", py_str_repr(t));
        }
    }
    format!("FQDN: looks like an IPv4 address ({})", py_str_repr(t))
}

// path / folder

fn bi_folder(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "folder", 1)?;
    match ObjectPath::try_parse(&s) {
        None => Ok(Value::Str(String::new())),
        Some(obj) => Ok(Value::Str(obj.folder.render())),
    }
}

fn bi_with_folder(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "with_folder", 1)?;
    let f = as_str(&args[1], "with_folder", 2)?;
    let obj = ObjectPath::try_parse(&p).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_folder: cannot parse path {}",
            py_str_repr(&p)
        ))
    })?;
    let new_folder = Folder::try_parse(&f).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_folder: cannot parse folder {}",
            py_str_repr(&f)
        ))
    })?;
    let result = ObjectPath {
        folder: new_folder,
        name: obj.name,
    };
    Ok(Value::Str(result.render()))
}

fn bi_with_name(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "with_name", 1)?;
    let n = as_str(&args[1], "with_name", 2)?.trim().to_string();
    if n.is_empty() {
        return Err(QueryError::builtin("with_name: new name must not be empty"));
    }
    match ObjectPath::try_parse(&p) {
        Some(obj) => {
            let result = ObjectPath {
                folder: obj.folder,
                name: n,
            };
            Ok(Value::Str(result.render()))
        }
        None => {
            if !p.is_empty() && !p.contains('/') {
                Ok(Value::Str(n))
            } else {
                Err(QueryError::builtin(format!(
                    "with_name: cannot parse path {}",
                    py_str_repr(&p)
                )))
            }
        }
    }
}

fn bi_in_partition(args: &[Value]) -> Result<Value, QueryError> {
    let p = coerce_pathlike(&args[0], "in_partition", 1)?;
    let part_text = as_str(&args[1], "in_partition", 2)?;
    let Some(obj) = ObjectPath::try_parse(&p) else {
        return Ok(Value::Bool(false));
    };
    let Some(target) = Partition::try_parse(&part_text) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(*obj.partition() == target))
}

fn bi_in_folder(args: &[Value]) -> Result<Value, QueryError> {
    let p = coerce_pathlike(&args[0], "in_folder", 1)?;
    let f = as_str(&args[1], "in_folder", 2)?;
    let obj = ObjectPath::try_parse(&p);
    let target_folder = Folder::try_parse(&f);
    let (Some(obj), Some(target)) = (obj, target_folder) else {
        return Ok(Value::Bool(false));
    };
    if obj.folder.partition != target.partition {
        return Ok(Value::Bool(false));
    }
    if target.segments.len() > obj.folder.segments.len() {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(
        obj.folder.segments[..target.segments.len()] == target.segments[..],
    ))
}

fn bi_can_see(args: &[Value]) -> Result<Value, QueryError> {
    let r_text = as_str(&args[0], "can_see", 1)?;
    let t_text = as_str(&args[1], "can_see", 2)?;
    let r = ObjectPath::try_parse(&r_text);
    let t = ObjectPath::try_parse(&t_text);
    let (Some(r), Some(t)) = (r, t) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(r.partition().can_see(t.partition())))
}

// port sets

#[derive(Clone, Copy)]
struct PortSeg {
    low: u32,
    high: u32,
}

/// Parse a port set into normalised segments, or `None`.
fn port_set_try_parse(text: &str) -> Option<Vec<PortSeg>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    if lower == "any" || lower == "*" || lower == "0" {
        return Some(vec![PortSeg {
            low: 0,
            high: 65535,
        }]);
    }
    let mut raw: Vec<PortSeg> = Vec::new();
    for chunk in text.split(',') {
        let piece = chunk.trim();
        if piece.is_empty() {
            continue;
        }
        if let Some((lo_text, hi_text)) = piece.split_once('-') {
            let lo: i64 = lo_text.trim().parse().ok()?;
            let hi: i64 = hi_text.trim().parse().ok()?;
            // PortSegment validation: 0 <= low, high <= 65535, low <= high.
            if lo < 0 || hi > 65535 || lo > hi {
                return None;
            }
            raw.push(PortSeg {
                low: lo as u32,
                high: hi as u32,
            });
        } else {
            let n: i64 = piece.parse().ok()?;
            if !(0..=65535).contains(&n) {
                return None;
            }
            raw.push(PortSeg {
                low: n as u32,
                high: n as u32,
            });
        }
    }
    if raw.is_empty() {
        return None;
    }
    Some(collapse_port_segments(raw))
}

/// Collapse adjacent / overlapping port segments.
fn collapse_port_segments(mut raw: Vec<PortSeg>) -> Vec<PortSeg> {
    raw.sort_by_key(|s| (s.low, s.high));
    let mut out: Vec<PortSeg> = vec![raw[0]];
    for seg in &raw[1..] {
        let last = out.last_mut().unwrap();
        if seg.low <= last.high + 1 {
            last.high = last.high.max(seg.high);
        } else {
            out.push(*seg);
        }
    }
    out
}

fn bi_port_set_contains(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port_set_contains", 1)?;
    let p = as_int(&args[1], "port_set_contains", 2)?;
    let Some(segs) = port_set_try_parse(&s) else {
        return Ok(Value::Bool(false));
    };
    let result = segs
        .iter()
        .any(|seg| i64::from(seg.low) <= p && p <= i64::from(seg.high));
    Ok(Value::Bool(result))
}

fn bi_port_set_count(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port_set_count", 1)?;
    match port_set_try_parse(&s) {
        None => Ok(Value::Null),
        Some(segs) => {
            let count: i64 = segs
                .iter()
                .map(|seg| i64::from(seg.high) - i64::from(seg.low) + 1)
                .sum();
            Ok(Value::Int(count))
        }
    }
}

fn bi_port_set_overlaps(args: &[Value]) -> Result<Value, QueryError> {
    let sa = as_str(&args[0], "port_set_overlaps", 1)?;
    let sb = as_str(&args[1], "port_set_overlaps", 2)?;
    let (Some(a), Some(b)) = (port_set_try_parse(&sa), port_set_try_parse(&sb)) else {
        return Ok(Value::Bool(false));
    };
    let result = a
        .iter()
        .any(|s| b.iter().any(|o| !(s.high < o.low || o.high < s.low)));
    Ok(Value::Bool(result))
}

// IP ranges

/// Parse an IP range into `(first, last)` as same-family `IpAddr`, or `None`.
fn ip_range_try_parse(text: &str) -> Option<(IpAddr, IpAddr)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if !text.contains('-') {
        let single = parse_ip_address(text)?;
        return Some((single, single));
    }
    let (lo_text, hi_text) = text.rsplit_once('-')?;
    let lo = parse_ip_address(lo_text.trim())?;
    let hi = parse_ip_address(hi_text.trim())?;
    // __post_init__: same family, first <= last.
    match (lo, hi) {
        (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {}
        _ => return None,
    }
    if ip_to_u128(lo) > ip_to_u128(hi) {
        return None;
    }
    Some((lo, hi))
}

fn bi_ip_range_to_cidrs(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_to_cidrs", 1)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Null);
    };
    let is_v4 = first.is_ipv4();
    let max_prefix: u8 = if is_v4 { 32 } else { 128 };
    let mut cidrs = Vec::new();
    summarize_range(ip_to_u128(first), ip_to_u128(last), max_prefix, &mut cidrs);
    let out: Vec<Value> = cidrs
        .into_iter()
        .map(|(addr, prefix)| Value::Str(format!("{}/{}", format_ip_int(addr, is_v4), prefix)))
        .collect();
    Ok(Value::List(out))
}

fn bi_ip_range_supernet(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_supernet", 1)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Null);
    };
    let is_v4 = first.is_ipv4();
    let max_prefix: u8 = if is_v4 { 32 } else { 128 };
    let lo = ip_to_u128(first);
    let hi = ip_to_u128(last);
    let mut cidrs = Vec::new();
    summarize_range(lo, hi, max_prefix, &mut cidrs);
    if cidrs.len() == 1 {
        let (addr, prefix) = cidrs[0];
        return Ok(Value::Str(format!(
            "{}/{}",
            format_ip_int(addr, is_v4),
            prefix
        )));
    }
    // Iteratively widen.
    for prefix in (0..=max_prefix).rev() {
        let (net, bcast) = network_at(lo, prefix, max_prefix);
        if net <= lo && bcast >= hi {
            return Ok(Value::Str(format!(
                "{}/{}",
                format_ip_int(net, is_v4),
                prefix
            )));
        }
    }
    Ok(Value::Str(if is_v4 {
        "0.0.0.0/0".to_string()
    } else {
        "::/0".to_string()
    }))
}

fn bi_ip_range_count(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_count", 1)?;
    match ip_range_try_parse(&s) {
        None => Ok(Value::Null),
        Some((first, last)) => {
            let count = ip_to_u128(last) - ip_to_u128(first) + 1;
            Ok(count_value(count))
        }
    }
}

fn bi_ip_range_contains(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_contains", 1)?;
    let a = as_str(&args[1], "ip_range_contains", 2)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Bool(false));
    };
    let Some(candidate) = parse_ip_address(&a) else {
        return Ok(Value::Bool(false));
    };
    if candidate.is_ipv4() != first.is_ipv4() {
        return Ok(Value::Bool(false));
    }
    let c = ip_to_u128(candidate);
    Ok(Value::Bool(ip_to_u128(first) <= c && c <= ip_to_u128(last)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `in_cidr(addr, net)` builtin as a bool (panics on a non-bool
    /// result, which the builtin never returns).
    fn in_cidr(addr: &str, net: &str) -> bool {
        match bi_in_cidr(&[Value::Str(addr.to_owned()), Value::Str(net.to_owned())]) {
            Ok(Value::Bool(b)) => b,
            other => panic!("in_cidr returned {other:?}"),
        }
    }

    #[test]
    fn in_cidr_matches_v4_ranges() {
        // `in_cidr(addr, cidr)` tests IPv4 membership, as used by
        // `select(in_cidr(.destination, "10.10.0.0/24"))`.
        assert!(in_cidr("10.10.0.5", "10.10.0.0/24"));
        assert!(in_cidr("10.10.255.1", "10.10.0.0/16"));
        assert!(!in_cidr("192.168.0.1", "10.10.0.0/16"));
        // A BIG-IP destination (addr:port) has its host extracted first.
        assert!(in_cidr("10.10.0.7:443", "10.10.0.0/24"));
        // A non-address is `false`, not an error.
        assert!(!in_cidr("not-an-ip", "10.0.0.0/8"));
    }

    #[test]
    fn in_cidr_matches_v6_ranges() {
        assert!(in_cidr("2001:db8::1", "2001:db8::/32"));
        assert!(!in_cidr("2001:db9::1", "2001:db8::/32"));
    }

    #[test]
    fn destination_parses_portless_ipv4_mapped_ipv6() {
        // `::ffff:10.1.1.1` (no port) — the trailing `.1` is part of the
        // address, not a port separator, so the whole thing parses as the
        // address with no port (issue 194).
        let d = Destination::try_parse("::ffff:10.1.1.1").expect("IPv4-mapped IPv6 parses");
        assert_eq!(d.port.port, 0, "no port");
        assert!(!d.ipv6_brackets);
        // A genuine unbracketed IPv6-with-port still splits at the trailing `.`.
        let d2 = Destination::try_parse("2001:db8::1.443").expect("IPv6 with port parses");
        assert_eq!(d2.port.port, 443);
    }

    #[test]
    fn with_port_on_portless_ipv4_mapped_ipv6_succeeds() {
        // The `with_port` builtin routes through the same parser (issue 194).
        let out = bi_with_port(&[Value::Str("::ffff:10.1.1.1".to_owned()), Value::Int(443)])
            .expect("with_port must parse the IPv4-mapped address");
        match out {
            Value::Str(s) => assert!(s.contains("443"), "port applied: {s}"),
            other => panic!("with_port returned {other:?}"),
        }
    }

    #[test]
    fn host_count_large_ipv6_is_not_negative() {
        // A /32 IPv6 network has 2^96-2 hosts — an `as i64`
        // truncation returned -2. The value must be a large, positive number
        // (a Float, since it exceeds i64), never negative garbage.
        match bi_host_count(&[Value::Str("2001:db8::/32".to_owned())]).unwrap() {
            Value::Float(f) => {
                assert!(f > 0.0, "count must be positive, got {f}");
                assert!(f > 7.9e28 * 0.99, "count ~= 2^96, got {f}");
            }
            other => panic!("expected Float for huge count, got {other:?}"),
        }
        // FP-guard: an IPv4 /24 still returns an exact Int (254 usable hosts).
        assert!(matches!(
            bi_host_count(&[Value::Str("10.0.0.0/24".to_owned())]).unwrap(),
            Value::Int(254)
        ));
    }

    #[test]
    fn ip_range_count_large_span_is_not_negative() {
        // A huge IPv6 range must not truncate to a negative i64.
        match bi_ip_range_count(&[Value::Str(
            "::-ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe".to_owned(),
        )])
        .unwrap()
        {
            Value::Float(f) => assert!(f > 0.0, "count must be positive, got {f}"),
            Value::Int(i) => assert!(i > 0, "count must be positive, got {i}"),
            other => panic!("unexpected {other:?}"),
        }
        // A small IPv4 range is an exact Int.
        assert!(matches!(
            bi_ip_range_count(&[Value::Str("10.0.0.1-10.0.0.10".to_owned())]).unwrap(),
            Value::Int(10)
        ));
    }
}
