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

//! Route-domain suffixes and masks shared by BIG-IP configuration and iRules.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Largest route-domain ID accepted by BIG-IP.
pub const MAX_ROUTE_DOMAIN: u32 = 65_534;

/// A malformed explicit route-domain suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteDomainError<'a> {
    /// Suffix is empty or contains a non-decimal character.
    Invalid(&'a str),
    /// Suffix is a decimal number outside the supported range.
    OutOfRange(&'a str),
}

/// Split a numeric route domain from a host, preserving explicit zero.
///
/// # Errors
/// Returns an error for a malformed or out-of-range suffix.
pub fn split_route_domain(text: &str) -> Result<(&str, Option<u32>), RouteDomainError<'_>> {
    let Some((host, digits)) = text.rsplit_once('%') else {
        return Ok((text, None));
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RouteDomainError::Invalid(digits));
    }
    let id = digits
        .parse::<u32>()
        .ok()
        .filter(|&id| id <= MAX_ROUTE_DOMAIN)
        .ok_or(RouteDomainError::OutOfRange(digits))?;
    Ok((host, Some(id)))
}

/// Read a prefix length or contiguous IP mask for the specified family.
#[must_use]
pub fn prefix_from_mask(mask: &str, ipv6: bool) -> Option<u8> {
    let width = if ipv6 { 128 } else { 32 };
    if let Ok(prefix) = mask.parse::<u8>() {
        return (prefix <= width).then_some(prefix);
    }
    let bits = if ipv6 {
        mask.parse::<Ipv6Addr>().ok()?.to_bits()
    } else {
        u128::from(mask.parse::<Ipv4Addr>().ok()?.to_bits()) << 96
    };
    let prefix = u8::try_from(bits.leading_ones()).ok()?;
    let expected = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
    (bits == expected).then_some(prefix)
}

/// A literal operand to the iRules address comparison command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressOperand {
    /// Literal IPv4 or IPv6 host.
    pub address: IpAddr,
    /// Explicit route domain, including zero.
    pub route_domain: Option<u32>,
    /// Explicit CIDR prefix or contiguous IPv4 mask.
    pub prefix: Option<u8>,
}

impl AddressOperand {
    /// Read the address comparison form measured in TMM.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (host, mask) = text
            .split_once('/')
            .map_or((text, None), |(host, mask)| (host, Some(mask)));
        let (host, route_domain) = split_route_domain(host).ok()?;
        let address: IpAddr = host.parse().ok()?;
        let prefix = match mask {
            Some(mask)
                if mask.bytes().all(|c| c.is_ascii_digit())
                    || (address.is_ipv4() && mask.contains('.')) =>
            {
                if mask.len() > 1 && mask.starts_with('0') && !mask.contains('.') {
                    return None;
                }
                Some(prefix_from_mask(mask, address.is_ipv6())?)
            }
            Some(_) => return None,
            None => None,
        };
        Some(Self {
            address,
            route_domain,
            prefix,
        })
    }
}
