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

//! Network facts and CIDR algebra: host ranges, `collapse_cidrs`, `supernet_of`, `subnet_of` and `overlaps`.

use tcl_bigip::value::Address;

use super::value_arguments::{typed_address, typed_network};
use super::{Net, parse_network};
use crate::errors::QueryError;
use crate::value::Value;
use ipnet::{Ipv4Net, Ipv6Net};
use std::net::{Ipv4Addr, Ipv6Addr};

// network info

pub(super) fn bi_prefix_length(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "prefix_length", 1)? {
        None => Ok(Value::Null),
        Some(n) => Ok(Value::Int(i64::from(n.prefix_len()))),
    }
}

pub(super) fn bi_network_address(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "network_address", 1)? {
        None => Ok(Value::Null),
        Some(n) => Ok(Value::Str(n.network_address_str())),
    }
}

pub(super) fn bi_broadcast_address(args: &[Value]) -> Result<Value, QueryError> {
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

pub(super) fn bi_first_host(args: &[Value]) -> Result<Value, QueryError> {
    match typed_network(&args[0], "first_host", 1)? {
        None => Ok(Value::Null),
        Some(n) => {
            let (first, _) = net_first_last_host(n);
            Ok(Value::Str(first))
        }
    }
}

pub(super) fn bi_last_host(args: &[Value]) -> Result<Value, QueryError> {
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
pub(super) fn count_value(count: u128) -> Value {
    i64::try_from(count).map_or_else(|_| Value::Float(count as f64), Value::Int)
}

pub(super) fn bi_host_count(args: &[Value]) -> Result<Value, QueryError> {
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

pub(super) fn bi_collapse_cidrs(args: &[Value]) -> Result<Value, QueryError> {
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
pub(super) fn summarize_range(first: u128, last: u128, max_prefix: u8, out: &mut Vec<(u128, u8)>) {
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

pub(super) fn bi_supernet_of(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "supernet_of", 1)?;
    let mut nets: Vec<Net> = Vec::new();
    for item in &items {
        if let Some(net) = typed_network(item, "supernet_of", 1)? {
            nets.push(net);
            continue;
        }
        if let Some(Address::Ip(ip)) = typed_address(item, "supernet_of")? {
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
pub(super) fn network_at(addr: u128, prefix: u8, max_prefix: u8) -> (u128, u128) {
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

pub(super) fn format_ip_int(v: u128, is_v4: bool) -> String {
    if is_v4 {
        Ipv4Addr::from(v as u32).to_string()
    } else {
        Ipv6Addr::from(v).to_string()
    }
}

pub(super) fn bi_subnet_of(args: &[Value]) -> Result<Value, QueryError> {
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

pub(super) fn bi_overlaps(args: &[Value]) -> Result<Value, QueryError> {
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
