//! `IPRange` value type — arbitrary first..last IP ranges.

use super::address::IPAddress;
use super::error::ValueError;
use super::network::Cidr;
use super::port::py_repr;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Inclusive `[first, last]` range of IPv4 or IPv6 addresses.
///
/// `first` and `last` must agree on address family (both v4 or both v6).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IPRange {
    /// First (lowest) address in the range.
    pub first: IpAddr,
    /// Last (highest) address in the range.
    pub last: IpAddr,
}

impl IPRange {
    /// Construct from two endpoints, validating family + ordering.
    ///
    /// # Errors
    /// Returns [`ValueError`] for mixed address families or `first > last`.
    pub fn new(first: IpAddr, last: IpAddr) -> Result<Self, ValueError> {
        match (first, last) {
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {}
            _ => {
                return Err(ValueError(format!(
                    "IPRange: mixed address families ({} / {})",
                    family_name(first),
                    family_name(last)
                )));
            }
        }
        if addr_int(first) > addr_int(last) {
            return Err(ValueError(format!(
                "IPRange: first > last ({first} > {last})"
            )));
        }
        Ok(Self { first, last })
    }

    /// Parse `"first-last"` (or a single address, yielding a one-IP range).
    ///
    /// # Errors
    /// Returns [`ValueError`] for malformed input or mixed families.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("IPRange: empty input".to_owned()));
        }
        if !text.contains('-') {
            let single = parse_ip(text)?;
            return Self::new(single, single);
        }
        // Split on the LAST hyphen.
        let idx = text.rfind('-').unwrap();
        let lo = parse_ip(text[..idx].trim())?;
        let hi = parse_ip(text[idx + 1..].trim())?;
        Self::new(lo, hi)
    }

    /// Like [`IPRange::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when the range is IPv4.
    #[must_use]
    pub fn is_ipv4(&self) -> bool {
        self.first.is_ipv4()
    }

    /// `true` when the range is IPv6.
    #[must_use]
    pub fn is_ipv6(&self) -> bool {
        self.first.is_ipv6()
    }

    /// Number of addresses in the range (inclusive).
    #[must_use]
    pub fn count(&self) -> u128 {
        addr_int(self.last) - addr_int(self.first) + 1
    }

    /// `true` when `addr` lies inside the range (same family required).
    #[must_use]
    pub fn contains_addr(&self, addr: IpAddr) -> bool {
        if self.first.is_ipv4() != addr.is_ipv4() {
            return false;
        }
        addr_int(self.first) <= addr_int(addr) && addr_int(addr) <= addr_int(self.last)
    }

    /// `true` when the given [`IPAddress`] lies inside the range.
    #[must_use]
    pub fn contains_ip(&self, ip: &IPAddress) -> bool {
        self.contains_addr(ip.addr)
    }

    /// `true` when the string parses to an address inside the range.
    #[must_use]
    pub fn contains_str(&self, s: &str) -> bool {
        match s.parse::<IpAddr>() {
            Ok(a) => self.contains_addr(a),
            Err(_) => false,
        }
    }

    /// `true` when `self` and `other` share at least one address.
    #[must_use]
    pub fn overlaps(&self, other: &IPRange) -> bool {
        if self.is_ipv4() != other.is_ipv4() {
            return false;
        }
        !(addr_int(self.last) < addr_int(other.first)
            || addr_int(other.last) < addr_int(self.first))
    }

    /// Decompose the range into the minimum CIDR set that exactly covers
    /// it.
    #[must_use]
    pub fn to_cidrs(&self) -> Vec<Cidr> {
        summarize(addr_int(self.first), addr_int(self.last), self.is_ipv4())
    }

    /// The smallest single CIDR that wholly contains the range.
    #[must_use]
    pub fn as_supernet(&self) -> Cidr {
        let cidrs = self.to_cidrs();
        if cidrs.len() == 1 {
            return cidrs[0];
        }
        let first = addr_int(self.first);
        let last = addr_int(self.last);
        let family_max: u32 = if self.is_ipv4() { 32 } else { 128 };
        for prefix in (0..=family_max).rev() {
            // `family_max` is 32 or 128, so every prefix fits in a u8.
            let p = u8::try_from(prefix).expect("prefix <= 128");
            let (net_lo, net_hi) = net_bounds(first, p, self.is_ipv4());
            if net_lo <= first && net_hi >= last {
                return make_cidr(net_lo, p, self.is_ipv4());
            }
        }
        // Unreachable: /0 always covers everything.
        make_cidr(0, 0, self.is_ipv4())
    }
}

impl fmt::Display for IPRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.first == self.last {
            write!(f, "{}", self.first)
        } else {
            write!(f, "{}-{}", self.first, self.last)
        }
    }
}

fn family_name(a: IpAddr) -> &'static str {
    match a {
        IpAddr::V4(_) => "IPv4Address",
        IpAddr::V6(_) => "IPv6Address",
    }
}

fn addr_int(a: IpAddr) -> u128 {
    match a {
        IpAddr::V4(v4) => u128::from(v4.to_bits()),
        IpAddr::V6(v6) => v6.to_bits(),
    }
}

fn parse_ip(text: &str) -> Result<IpAddr, ValueError> {
    text.parse().map_err(|_| {
        ValueError(format!(
            "{} does not appear to be an IPv4 or IPv6 address",
            py_repr(text)
        ))
    })
}

fn make_cidr(network_int: u128, prefix: u8, is_v4: bool) -> Cidr {
    if is_v4 {
        // The v4 path only ever receives network ints masked to 32 bits.
        Cidr::V4 {
            network: Ipv4Addr::from_bits(
                u32::try_from(network_int).expect("v4 network masked to 32 bits"),
            ),
            prefix,
        }
    } else {
        Cidr::V6 {
            network: Ipv6Addr::from_bits(network_int),
            prefix,
        }
    }
}

/// Return `(network_low, network_high)` for the masked CIDR containing
/// `addr` at `prefix`, for the given family.
fn net_bounds(addr: u128, prefix: u8, is_v4: bool) -> (u128, u128) {
    let bits: u32 = if is_v4 { 32 } else { 128 };
    if prefix == 0 {
        let max = if is_v4 {
            u128::from(u32::MAX)
        } else {
            u128::MAX
        };
        return (0, max);
    }
    let host_bits = bits - u32::from(prefix);
    if host_bits == 0 {
        return (addr, addr);
    }
    let lo = (addr >> host_bits) << host_bits;
    let hi = lo | ((1u128 << host_bits) - 1);
    (lo, hi)
}

/// Count the number of right-hand zero bits of `number`, capped at `bits`.
fn count_righthand_zero_bits(number: u128, bits: u32) -> u32 {
    if number == 0 {
        bits
    } else {
        number.trailing_zeros().min(bits)
    }
}

/// Hand-rolled `summarize_address_range` over integer endpoints.
fn summarize(first: u128, last: u128, is_v4: bool) -> Vec<Cidr> {
    let ip_bits: u32 = if is_v4 { 32 } else { 128 };
    let all_ones = if is_v4 {
        u128::from(u32::MAX)
    } else {
        u128::MAX
    };
    let mut out = Vec::new();
    let mut first_int = first;
    let last_int = last;
    while first_int <= last_int {
        // ``(last - first + 1).bit_length() - 1`` — bits of headroom.
        let span = last_int - first_int + 1;
        let span_bits = (128 - span.leading_zeros()).saturating_sub(1);
        let nbits = count_righthand_zero_bits(first_int, ip_bits).min(span_bits);
        // `ip_bits` is 32 or 128 and `nbits <= ip_bits`, so the prefix fits in a u8.
        let prefix = u8::try_from(ip_bits - nbits).expect("prefix <= 128");
        out.push(make_cidr(first_int, prefix, is_v4));
        // ``first_int += 1 << nbits``; break when we've consumed the whole
        // address space (the ``first_int - 1 == _ALL_ONES`` all-ones guard).
        let step = 1u128 << nbits;
        match first_int.checked_add(step) {
            Some(next) => {
                first_int = next;
                if first_int.wrapping_sub(1) == all_ones {
                    break;
                }
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_display() {
        let r = IPRange::parse("192.168.9.77-192.168.9.83").unwrap();
        assert_eq!(r.to_string(), "192.168.9.77-192.168.9.83");
        assert_eq!(r.count(), 7);
        assert!(r.is_ipv4());
    }

    #[test]
    fn single_address_range() {
        let r = IPRange::parse("10.0.0.1").unwrap();
        assert_eq!(r.first, r.last);
        assert_eq!(r.to_string(), "10.0.0.1");
        assert_eq!(r.count(), 1);
    }

    #[test]
    fn rejects_mixed_and_reversed() {
        assert!(IPRange::parse("10.0.0.5-10.0.0.1").is_err());
        // Mixed families.
        assert!(IPRange::new("10.0.0.1".parse().unwrap(), "2001:db8::1".parse().unwrap()).is_err());
    }

    #[test]
    fn membership_and_overlap() {
        let r = IPRange::parse("10.0.0.0-10.0.0.255").unwrap();
        assert!(r.contains_str("10.0.0.50"));
        assert!(!r.contains_str("10.0.1.0"));
        let other = IPRange::parse("10.0.0.200-10.0.1.0").unwrap();
        assert!(r.overlaps(&other));
        let far = IPRange::parse("10.0.2.0-10.0.2.5").unwrap();
        assert!(!r.overlaps(&far));
    }

    #[test]
    fn to_cidrs_matches_python_example() {
        // summarize_address_range(192.0.2.0, 192.0.2.130)
        let r = IPRange::parse("192.0.2.0-192.0.2.130").unwrap();
        let cidrs = r.to_cidrs();
        let rendered: Vec<String> = cidrs
            .iter()
            .map(|c| match c {
                Cidr::V4 { network, prefix } => format!("{network}/{prefix}"),
                Cidr::V6 { network, prefix } => format!("{network}/{prefix}"),
            })
            .collect();
        assert_eq!(
            rendered,
            vec!["192.0.2.0/25", "192.0.2.128/31", "192.0.2.130/32"]
        );
    }

    #[test]
    fn supernet() {
        let r = IPRange::parse("192.168.9.77-192.168.9.83").unwrap();
        let sup = r.as_supernet();
        // smallest single CIDR covering 77..83 is 192.168.9.64/27.
        match sup {
            Cidr::V4 { network, prefix } => {
                assert_eq!(format!("{network}/{prefix}"), "192.168.9.64/27");
            }
            Cidr::V6 { .. } => panic!("expected v4"),
        }
    }
}
