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

//! The typed value layer: IP address, FQDN, port, partition, folder, object path, destination and network.

use super::{Net, parse_ip_address, parse_network};
use crate::builtins::as_str;
use crate::errors::QueryError;
use crate::value::Value;
use std::net::IpAddr;

// Typed value layer: IPAddress / FQDN, RouteDomain, Port, Partition,
// Folder, ObjectPath, Destination, Network, IPRange, PortSet.

const PART_VALID: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-.";

fn part_chars_ok(s: &str) -> bool {
    s.chars().all(|c| PART_VALID.contains(c))
}

/// An IP plus an optional `%rd` route domain.
#[derive(Clone)]
pub(super) struct TypedIp {
    pub(super) addr: IpAddr,
    pub(super) route_domain: Option<u64>,
}

impl TypedIp {
    /// Parse `text` into a `TypedIp` (or `None`).
    pub(super) fn try_parse(text: &str) -> Option<TypedIp> {
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
    !(labels.len() == 4 && labels.iter().all(|l| l.chars().all(|c| c.is_ascii_digit())))
}

/// `Address` sum type result.
pub(super) enum TypedAddress {
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
pub(super) fn parse_address(text: &str) -> Option<TypedAddress> {
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
pub(super) struct TypedPort {
    pub(super) port: u16,
    pub(super) spelling: String,
}

impl TypedPort {
    pub(super) fn try_parse(text: &str) -> Option<TypedPort> {
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

    pub(super) fn is_any(&self) -> bool {
        self.port == 0
    }

    pub(super) fn render(&self) -> String {
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
pub(super) struct Partition {
    pub(super) name: String, // canonical with leading slash
}

impl Partition {
    pub(super) fn try_parse(text: &str) -> Option<Partition> {
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

    pub(super) fn can_see(&self, other: &Partition) -> bool {
        self == other || other.is_common()
    }
}

/// A folder: a partition plus its nested path segments.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Folder {
    pub(super) partition: Partition,
    pub(super) segments: Vec<String>,
}

impl Folder {
    pub(super) fn try_parse(text: &str) -> Option<Folder> {
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

    pub(super) fn render(&self) -> String {
        if self.is_root() {
            self.partition.name.clone()
        } else {
            format!("{}/{}", self.partition.name, self.segments.join("/"))
        }
    }
}

/// An object path: a folder plus the object's name.
pub(super) struct ObjectPath {
    pub(super) folder: Folder,
    pub(super) name: String,
}

impl ObjectPath {
    pub(super) fn try_parse(text: &str) -> Option<ObjectPath> {
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

    pub(super) fn partition(&self) -> &Partition {
        &self.folder.partition
    }

    pub(super) fn render(&self) -> String {
        if self.folder.is_root() {
            format!("{}/{}", self.folder.partition.name, self.name)
        } else {
            format!("{}/{}", self.folder.render(), self.name)
        }
    }
}

/// A parsed BIG-IP destination (address, port, folder, route domain).
pub(super) struct Destination {
    pub(super) address: TypedAddress,
    pub(super) port: TypedPort,
    pub(super) folder: Option<Folder>,
    pub(super) route_domain: Option<u64>,
    pub(super) ipv6_brackets: bool,
    pub(super) port_separator: char,
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
    pub(super) fn try_parse(text: &str) -> Option<Destination> {
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
                    // the split doesn't yield a valid IP + port.
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

    pub(super) fn render(&self) -> String {
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
pub(super) fn split_destination(value: &str) -> (String, String, String, String) {
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
pub(super) fn rebuild_destination(
    partition: &str,
    address: &str,
    route_domain: &str,
    port: &str,
) -> String {
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
pub(super) fn typed_address(value: &Value, name: &str) -> Result<Option<TypedAddress>, QueryError> {
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
pub(super) fn typed_network(
    value: &Value,
    name: &str,
    arg: usize,
) -> Result<Option<Net>, QueryError> {
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
