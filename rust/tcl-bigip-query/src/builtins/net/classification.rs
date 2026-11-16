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

//! Address classification against the IANA special-purpose range tables, IPv4 and IPv6.

use ipnet::Ipv6Net;
use std::net::IpAddr;

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

pub(super) fn addr_is_ipv4(a: IpAddr) -> bool {
    a.is_ipv4()
}

pub(super) fn addr_is_ipv6(a: IpAddr) -> bool {
    a.is_ipv6()
}

pub(super) fn addr_is_private(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_private(u32::from(v)),
        IpAddr::V6(v) => v6_is_private(u128::from(v)),
    }
}

pub(super) fn addr_is_loopback(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_loopback(u32::from(v)),
        IpAddr::V6(v) => v6_is_loopback(u128::from(v)),
    }
}

pub(super) fn addr_is_unspecified(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_unspecified(u32::from(v)),
        IpAddr::V6(v) => v6_is_unspecified(u128::from(v)),
    }
}

pub(super) fn addr_is_multicast(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_multicast(u32::from(v)),
        IpAddr::V6(v) => v6_is_multicast(u128::from(v)),
    }
}

pub(super) fn addr_is_link_local(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v) => v4_is_link_local(u32::from(v)),
        IpAddr::V6(v) => v6_is_link_local(u128::from(v)),
    }
}

pub(super) fn addr_is_reserved(a: IpAddr) -> bool {
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
pub(super) fn addr_is_public(a: IpAddr) -> bool {
    addr_is_global(a)
        && !addr_is_multicast(a)
        && !addr_is_link_local(a)
        && !addr_is_loopback(a)
        && !addr_is_unspecified(a)
        && !addr_is_reserved(a)
}

/// `IPAddress.is_documentation` — the F5 typed layer checks canonical ranges.
pub(super) fn addr_is_documentation(a: IpAddr) -> bool {
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
