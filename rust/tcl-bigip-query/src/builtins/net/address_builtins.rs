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

//! Builtins that read one address: `ip`, `net`, `host`, `port`, route domains and the classification predicates.

use super::classification::{
    addr_is_documentation, addr_is_ipv4, addr_is_ipv6, addr_is_link_local, addr_is_loopback,
    addr_is_multicast, addr_is_private, addr_is_public, addr_is_reserved, addr_is_unspecified,
};
use super::value_arguments::{destination_fields, typed_address};
use super::{Net, addr_value_error, net_value_error, parse_ip_address, parse_network, py_str_repr};
use crate::builtins::{as_str, type_name};
use crate::errors::QueryError;
use crate::value::Value;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tcl_bigip::value::{Address, Destination, IPAddress, Network, RouteDomain};

// Builtins

pub(super) fn bi_ip(args: &[Value]) -> Result<Value, QueryError> {
    if args.len() == 1 {
        let s = as_str(&args[0], "ip", 1)?;
        let (addr, _, _) = destination_fields(&s);
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
    let (src_addr, _, _) = destination_fields(&src_str);
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
            IpAddr::V4(Ipv4Addr::from(new_int))
        }
        (Net::V6(n), IpAddr::V6(s)) => {
            let netmask: u128 = if n.prefix_len() == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(n.prefix_len()))
            };
            let host_bits = u128::from(s) & !netmask;
            let new_int = u128::from(n.network()) | host_bits;
            IpAddr::V6(Ipv6Addr::from(new_int))
        }
        _ => unreachable!("family mismatch handled above"),
    };
    let mut endpoint = Destination::try_parse(&src_str).ok_or_else(|| {
        QueryError::builtin(format!(
            "ip: invalid source address {}",
            py_str_repr(&src_str)
        ))
    })?;
    endpoint.address = Address::Ip(IPAddress::new(new_addr, None));
    Ok(Value::Str(endpoint.to_string()))
}

pub(super) fn bi_ip_translate(args: &[Value]) -> Result<Value, QueryError> {
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
    let (bare_addr, _, _) = destination_fields(&addr_str);
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

pub(super) fn ip_to_u128(a: IpAddr) -> u128 {
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

pub(super) fn bi_net(args: &[Value]) -> Result<Value, QueryError> {
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

pub(super) fn bi_host(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "host", 1)?;
    let (addr, _, _) = destination_fields(&s);
    Ok(Value::Str(addr))
}

pub(super) fn bi_port(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port", 1)?;
    let (_, _, port) = destination_fields(&s);
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

pub(super) fn bi_in_cidr(args: &[Value]) -> Result<Value, QueryError> {
    let addr_s = as_str(&args[0], "in_cidr", 1)?;
    let net_s = as_str(&args[1], "in_cidr", 2)?;
    let (host, _, _) = destination_fields(&addr_s);
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

pub(super) fn bi_route_domain(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "route_domain", 1)?;
    let (_, rd, _) = destination_fields(&s);
    if rd.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Str(rd))
    }
}

pub(super) fn bi_with_route_domain(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_route_domain", 1)?;
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
    let domain = if new_rd.is_empty() {
        None
    } else {
        Some(RouteDomain::parse(&new_rd).map_err(|error| QueryError::builtin(error.to_string()))?)
    };
    if let Some(network) = Network::try_parse(&s) {
        return Ok(Value::Str(network.with_route_domain(domain.as_ref())));
    }
    let mut destination = Destination::try_parse(&s).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_route_domain: cannot parse {}",
            py_str_repr(&s)
        ))
    })?;
    destination.route_domain = domain;
    Ok(Value::Str(destination.to_string()))
}

// predicates

fn addr_predicate(value: &Value, name: &str, f: fn(IpAddr) -> bool) -> Result<Value, QueryError> {
    let a = typed_address(value, name)?;
    Ok(Value::Bool(match a {
        Some(Address::Ip(ip)) => f(ip.addr),
        _ => false,
    }))
}

pub(super) fn bi_is_ipv4(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_ipv4", addr_is_ipv4)
}

pub(super) fn bi_is_ipv6(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_ipv6", addr_is_ipv6)
}

pub(super) fn bi_is_fqdn(args: &[Value]) -> Result<Value, QueryError> {
    let a = typed_address(&args[0], "is_fqdn")?;
    Ok(Value::Bool(matches!(a, Some(Address::Fqdn(_)))))
}

pub(super) fn bi_is_private(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_private", addr_is_private)
}

pub(super) fn bi_is_loopback(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_loopback", addr_is_loopback)
}

pub(super) fn bi_is_unspecified(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_unspecified", addr_is_unspecified)
}

pub(super) fn bi_is_multicast(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_multicast", addr_is_multicast)
}

pub(super) fn bi_is_link_local(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_link_local", addr_is_link_local)
}

pub(super) fn bi_is_reserved(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_reserved", addr_is_reserved)
}

pub(super) fn bi_is_public(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_public", addr_is_public)
}

pub(super) fn bi_is_documentation(args: &[Value]) -> Result<Value, QueryError> {
    addr_predicate(&args[0], "is_documentation", addr_is_documentation)
}

pub(super) fn bi_is_wildcard_port(args: &[Value]) -> Result<Value, QueryError> {
    if matches!(&args[0], Value::Null) {
        return Ok(Value::Bool(false));
    }
    let s = as_str(&args[0], "is_wildcard_port", 1)?;
    let result = Destination::try_parse(&s).is_some_and(|d| d.port.is_any());
    Ok(Value::Bool(result))
}
