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

//! Small, standalone WASM bridge for reports with or without the query console.

#[path = "../../tcl-syntax/src/irules_address.rs"]
mod irules_address;

use irules_address::AddressOperand;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU8, Ordering};

const CAPACITY: usize = 256;
static INPUT: [AtomicU8; CAPACITY] = [const { AtomicU8::new(0) }; CAPACITY];

#[unsafe(no_mangle)]
extern "C" fn address_input() -> *const AtomicU8 {
    INPUT.as_ptr()
}

fn read<R>(length: usize, parse: impl FnOnce(&str) -> Option<R>) -> Option<R> {
    let mut bytes = [0; CAPACITY];
    let bytes = bytes.get_mut(..length)?;
    for (byte, input) in bytes.iter_mut().zip(INPUT.iter()) {
        *byte = input.load(Ordering::Relaxed);
    }
    parse(std::str::from_utf8(bytes).ok()?)
}

#[unsafe(no_mangle)]
extern "C" fn describe_address(length: usize) -> u32 {
    read(length, |text| {
        let value = AddressOperand::parse(text)?;
        let (family, width) = if value.address.is_ipv4() {
            (4, 32)
        } else {
            (6, 128)
        };
        let prefix = u32::from(value.prefix.unwrap_or(width));
        let route_domain = value.route_domain.map_or(0, |domain| domain + 1);
        Some(family | (prefix << 8) | (route_domain << 16))
    })
    .unwrap_or(0)
}

#[unsafe(no_mangle)]
extern "C" fn match_addresses(length: usize, overlap: bool) -> bool {
    read(length, |text| {
        let (first, second) = text.split_once('\0')?;
        let first = AddressOperand::parse(first)?;
        let second = AddressOperand::parse(second)?;
        let (first_bits, second_bits, width) = match (first.address, second.address) {
            (IpAddr::V4(a), IpAddr::V4(b)) => {
                (u128::from(a.to_bits()), u128::from(b.to_bits()), 32)
            }
            (IpAddr::V6(a), IpAddr::V6(b)) => (a.to_bits(), b.to_bits(), 128),
            _ => return Some(false),
        };
        let mut prefix = second.prefix.unwrap_or(width);
        if overlap {
            prefix = prefix.min(first.prefix.unwrap_or(width));
        }
        let host_bits = u32::from(width - prefix);
        Some(first_bits.checked_shr(host_bits) == second_bits.checked_shr(host_bits))
    })
    .unwrap_or(false)
}
