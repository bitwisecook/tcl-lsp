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
    // address with no port.
    let d = Destination::try_parse("::ffff:10.1.1.1").expect("IPv4-mapped IPv6 parses");
    assert_eq!(d.port.port, 0, "no port");
    assert!(!d.ipv6_brackets);
    // A genuine unbracketed IPv6-with-port still splits at the trailing `.`.
    let d2 = Destination::try_parse("2001:db8::1.443").expect("IPv6 with port parses");
    assert_eq!(d2.port.port, 443);
}

#[test]
fn with_port_on_portless_ipv4_mapped_ipv6_succeeds() {
    // The `with_port` builtin routes through the same parser.
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
