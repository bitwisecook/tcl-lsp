//! IP-address classification predicates that match `ipaddress`
//! results exactly.
//!
//! `std::net` lacks stable predicates for `is_private` / `is_global` /
//! `is_reserved`, so these are hand-rolled against the RFC ranges
//! (and exception lists) that define each classification, including
//! the IPv4-mapped IPv6 delegation. The constant lists hold the
//! per-family special-purpose-address ranges.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// `(network_u32, prefix_len)` for an IPv4 CIDR.
type V4Net = (u32, u8);
/// `(network_u128, prefix_len)` for an IPv6 CIDR.
type V6Net = (u128, u8);

fn v4_in(addr: u32, (net, prefix): V4Net) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u32::MAX << (32 - u32::from(prefix));
    (addr & mask) == (net & mask)
}

fn v6_in(addr: u128, (net, prefix): V6Net) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u128::MAX << (128 - u32::from(prefix));
    (addr & mask) == (net & mask)
}

// IPv4 constants (mirror `IPv4Address._constants`).
const V4_PRIVATE: &[V4Net] = &[
    (0x0000_0000, 8),  // 0.0.0.0/8
    (0x0A00_0000, 8),  // 10.0.0.0/8
    (0x7F00_0000, 8),  // 127.0.0.0/8
    (0xA9FE_0000, 16), // 169.254.0.0/16
    (0xAC10_0000, 12), // 172.16.0.0/12
    (0xC000_0000, 24), // 192.0.0.0/24
    (0xC000_00AA, 31), // 192.0.0.170/31
    (0xC000_0200, 24), // 192.0.2.0/24
    (0xC0A8_0000, 16), // 192.168.0.0/16
    (0xC612_0000, 15), // 198.18.0.0/15
    (0xC633_6400, 24), // 198.51.100.0/24
    (0xCB00_7100, 24), // 203.0.113.0/24
    (0xF000_0000, 4),  // 240.0.0.0/4
    (0xFFFF_FFFF, 32), // 255.255.255.255/32
];
const V4_PRIVATE_EXCEPTIONS: &[V4Net] = &[
    (0xC000_0009, 32), // 192.0.0.9/32
    (0xC000_000A, 32), // 192.0.0.10/32
];
const V4_PUBLIC: V4Net = (0x6440_0000, 10); // 100.64.0.0/10
const V4_RESERVED: V4Net = (0xF000_0000, 4); // 240.0.0.0/4
const V4_LINKLOCAL: V4Net = (0xA9FE_0000, 16); // 169.254.0.0/16
const V4_LOOPBACK: V4Net = (0x7F00_0000, 8); // 127.0.0.0/8
const V4_MULTICAST: V4Net = (0xE000_0000, 4); // 224.0.0.0/4

fn v4_is_private(addr: u32) -> bool {
    V4_PRIVATE.iter().any(|&n| v4_in(addr, n))
        && V4_PRIVATE_EXCEPTIONS.iter().all(|&n| !v4_in(addr, n))
}

fn v4_is_global(addr: u32) -> bool {
    !v4_in(addr, V4_PUBLIC) && !v4_is_private(addr)
}

fn v4_is_reserved(addr: u32) -> bool {
    v4_in(addr, V4_RESERVED)
}

fn v4_is_multicast(addr: u32) -> bool {
    v4_in(addr, V4_MULTICAST)
}

fn v4_is_link_local(addr: u32) -> bool {
    v4_in(addr, V4_LINKLOCAL)
}

fn v4_is_loopback(addr: u32) -> bool {
    v4_in(addr, V4_LOOPBACK)
}

fn v4_is_unspecified(addr: u32) -> bool {
    addr == 0
}

// IPv6 constants (mirror `IPv6Address._constants`).
const V6_PRIVATE: &[V6Net] = &[
    (0x0000_0000_0000_0000_0000_0000_0000_0001, 128), // ::1/128
    (0x0000_0000_0000_0000_0000_0000_0000_0000, 128), // ::/128
    (0x0000_0000_0000_0000_0000_FFFF_0000_0000, 96),  // ::ffff:0.0.0.0/96
    (0x0064_FF9B_0001_0000_0000_0000_0000_0000, 48),  // 64:ff9b:1::/48
    (0x0100_0000_0000_0000_0000_0000_0000_0000, 64),  // 100::/64
    (0x2001_0000_0000_0000_0000_0000_0000_0000, 23),  // 2001::/23
    (0x2001_0DB8_0000_0000_0000_0000_0000_0000, 32),  // 2001:db8::/32
    (0x2002_0000_0000_0000_0000_0000_0000_0000, 16),  // 2002::/16
    (0x3FFF_0000_0000_0000_0000_0000_0000_0000, 20),  // 3fff::/20
    (0xFC00_0000_0000_0000_0000_0000_0000_0000, 7),   // fc00::/7
    (0xFE80_0000_0000_0000_0000_0000_0000_0000, 10),  // fe80::/10
];
const V6_PRIVATE_EXCEPTIONS: &[V6Net] = &[
    (0x2001_0001_0000_0000_0000_0000_0000_0001, 128), // 2001:1::1/128
    (0x2001_0001_0000_0000_0000_0000_0000_0002, 128), // 2001:1::2/128
    (0x2001_0003_0000_0000_0000_0000_0000_0000, 32),  // 2001:3::/32
    (0x2001_0004_0112_0000_0000_0000_0000_0000, 48),  // 2001:4:112::/48
    (0x2001_0020_0000_0000_0000_0000_0000_0000, 28),  // 2001:20::/28
    (0x2001_0030_0000_0000_0000_0000_0000_0000, 28),  // 2001:30::/28
];
const V6_RESERVED: &[V6Net] = &[
    (0x0000_0000_0000_0000_0000_0000_0000_0000, 8), // ::/8
    (0x0100_0000_0000_0000_0000_0000_0000_0000, 8), // 100::/8
    (0x0200_0000_0000_0000_0000_0000_0000_0000, 7), // 200::/7
    (0x0400_0000_0000_0000_0000_0000_0000_0000, 6), // 400::/6
    (0x0800_0000_0000_0000_0000_0000_0000_0000, 5), // 800::/5
    (0x1000_0000_0000_0000_0000_0000_0000_0000, 4), // 1000::/4
    (0x4000_0000_0000_0000_0000_0000_0000_0000, 3), // 4000::/3
    (0x6000_0000_0000_0000_0000_0000_0000_0000, 3), // 6000::/3
    (0x8000_0000_0000_0000_0000_0000_0000_0000, 3), // 8000::/3
    (0xA000_0000_0000_0000_0000_0000_0000_0000, 3), // a000::/3
    (0xC000_0000_0000_0000_0000_0000_0000_0000, 3), // c000::/3
    (0xE000_0000_0000_0000_0000_0000_0000_0000, 4), // e000::/4
    (0xF000_0000_0000_0000_0000_0000_0000_0000, 5), // f000::/5
    (0xF800_0000_0000_0000_0000_0000_0000_0000, 6), // f800::/6
    (0xFE00_0000_0000_0000_0000_0000_0000_0000, 9), // fe00::/9
];
const V6_LINKLOCAL: V6Net = (0xFE80_0000_0000_0000_0000_0000_0000_0000, 10); // fe80::/10
const V6_MULTICAST: V6Net = (0xFF00_0000_0000_0000_0000_0000_0000_0000, 8); // ff00::/8
const V6_MAPPED: V6Net = (0x0000_0000_0000_0000_0000_FFFF_0000_0000, 96); // ::ffff:0:0/96

/// Return the embedded IPv4 address of an IPv4-mapped IPv6 address
/// (`::ffff:a.b.c.d`), per the IPv4-mapped IPv6 address rule.
fn ipv4_mapped(addr: u128) -> Option<u32> {
    if v6_in(addr, V6_MAPPED) {
        // The low 32 bits are masked off, so the value always fits in a u32.
        Some(u32::try_from(addr & 0xFFFF_FFFF).expect("masked to 32 bits"))
    } else {
        None
    }
}

fn v6_is_private(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_private(mapped);
    }
    V6_PRIVATE.iter().any(|&n| v6_in(addr, n))
        && V6_PRIVATE_EXCEPTIONS.iter().all(|&n| !v6_in(addr, n))
}

fn v6_is_global(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_global(mapped);
    }
    !v6_is_private(addr)
}

fn v6_is_reserved(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_reserved(mapped);
    }
    V6_RESERVED.iter().any(|&n| v6_in(addr, n))
}

fn v6_is_multicast(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_multicast(mapped);
    }
    v6_in(addr, V6_MULTICAST)
}

fn v6_is_link_local(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_link_local(mapped);
    }
    v6_in(addr, V6_LINKLOCAL)
}

fn v6_is_loopback(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_loopback(mapped);
    }
    addr == 1
}

fn v6_is_unspecified(addr: u128) -> bool {
    if let Some(mapped) = ipv4_mapped(addr) {
        return v4_is_unspecified(mapped);
    }
    addr == 0
}

/// `Ipv4Addr::is_loopback` etc. agree for these, but to keep one
/// code path we route everything through the integer predicates.
#[must_use]
pub(crate) fn is_loopback(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_loopback(a.to_bits()),
        IpAddr::V6(a) => v6_is_loopback(a.to_bits()),
    }
}

/// `true` when the address is in a private (not-globally-reachable) range.
#[must_use]
pub(crate) fn is_private(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_private(a.to_bits()),
        IpAddr::V6(a) => v6_is_private(a.to_bits()),
    }
}

/// `true` when the address is unspecified (`0.0.0.0` / `::`).
#[must_use]
pub(crate) fn is_unspecified(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_unspecified(a.to_bits()),
        IpAddr::V6(a) => v6_is_unspecified(a.to_bits()),
    }
}

/// `true` for multicast addresses.
#[must_use]
pub(crate) fn is_multicast(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_multicast(a.to_bits()),
        IpAddr::V6(a) => v6_is_multicast(a.to_bits()),
    }
}

/// `true` for link-local addresses.
#[must_use]
pub(crate) fn is_link_local(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_link_local(a.to_bits()),
        IpAddr::V6(a) => v6_is_link_local(a.to_bits()),
    }
}

/// `true` for IANA-reserved addresses.
#[must_use]
pub(crate) fn is_reserved(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_reserved(a.to_bits()),
        IpAddr::V6(a) => v6_is_reserved(a.to_bits()),
    }
}

/// `true` when the address is globally reachable.
#[must_use]
pub(crate) fn is_global(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(a) => v4_is_global(a.to_bits()),
        IpAddr::V6(a) => v6_is_global(a.to_bits()),
    }
}

/// `true` when `addr` is inside the inclusive IPv4 documentation ranges.
#[must_use]
pub(crate) fn v4_is_documentation(addr: Ipv4Addr) -> bool {
    let a = addr.to_bits();
    v4_in(a, (0xC000_0200, 24)) // 192.0.2.0/24
        || v4_in(a, (0xC633_6400, 24)) // 198.51.100.0/24
        || v4_in(a, (0xCB00_7100, 24)) // 203.0.113.0/24
}

/// `true` when `addr` is inside the IPv6 documentation range `2001:db8::/32`.
#[must_use]
pub(crate) fn v6_is_documentation(addr: Ipv6Addr) -> bool {
    v6_in(
        addr.to_bits(),
        (0x2001_0DB8_0000_0000_0000_0000_0000_0000, 32),
    )
}

// Network-level predicates: a network is in a category when BOTH its
// network and broadcast (last) addresses are. `is_global` at the network
// level is simply `not is_private` (no 100.64 carve-out).

/// `true` when both endpoints of the IPv4 network are private.
#[must_use]
pub(crate) fn v4_net_is_private(net: u32, last: u32) -> bool {
    let in_priv = V4_PRIVATE.iter().any(|&n| v4_in(net, n) && v4_in(last, n));
    let in_exc = V4_PRIVATE_EXCEPTIONS
        .iter()
        .any(|&n| v4_in(net, n) || v4_in(last, n));
    in_priv && !in_exc
}

/// `true` when both endpoints of the IPv6 network are private.
#[must_use]
pub(crate) fn v6_net_is_private(net: u128, last: u128) -> bool {
    // Network-level `is_private` does not delegate ipv4_mapped; it checks
    // both endpoints directly against the v6 private list.
    let in_priv = V6_PRIVATE.iter().any(|&n| v6_in(net, n) && v6_in(last, n));
    let in_exc = V6_PRIVATE_EXCEPTIONS
        .iter()
        .any(|&n| v6_in(net, n) || v6_in(last, n));
    in_priv && !in_exc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn classifies_v4() {
        assert!(is_private(ip("10.0.0.1")));
        assert!(is_private(ip("192.168.1.1")));
        assert!(is_private(ip("172.16.0.1")));
        assert!(!is_private(ip("100.64.0.1")));
        assert!(!is_global(ip("100.64.0.1")));
        assert!(!is_private(ip("8.8.8.8")));
        assert!(is_global(ip("8.8.8.8")));
        assert!(is_link_local(ip("169.254.1.1")));
        assert!(is_loopback(ip("127.0.0.1")));
        assert!(is_multicast(ip("224.0.0.1")));
        assert!(is_global(ip("224.0.0.1")));
        assert!(is_reserved(ip("240.0.0.1")));
        assert!(is_unspecified(ip("0.0.0.0")));
        assert!(is_reserved(ip("255.255.255.255")));
        // 192.0.0.9 / .10 are private-exceptions -> not private, global.
        assert!(!is_private(ip("192.0.0.9")));
        assert!(is_global(ip("192.0.0.9")));
        assert!(is_private(ip("192.0.0.1")));
    }

    #[test]
    fn classifies_v6() {
        assert!(is_loopback(ip("::1")));
        assert!(is_unspecified(ip("::")));
        assert!(is_link_local(ip("fe80::1")));
        assert!(is_multicast(ip("ff02::1")));
        assert!(is_private(ip("fc00::1")));
        assert!(is_private(ip("2001:db8::1")));
        assert!(v6_is_documentation("2001:db8::1".parse().unwrap()));
        // IPv4-mapped delegates to the v4 semantics.
        assert!(is_global(ip("::ffff:8.8.8.8")));
        assert!(is_private(ip("::ffff:10.0.0.1")));
    }

    #[test]
    fn documentation_v4() {
        assert!(v4_is_documentation("192.0.2.5".parse().unwrap()));
        assert!(v4_is_documentation("198.51.100.5".parse().unwrap()));
        assert!(v4_is_documentation("203.0.113.5".parse().unwrap()));
        assert!(!v4_is_documentation("10.0.0.1".parse().unwrap()));
    }
}
