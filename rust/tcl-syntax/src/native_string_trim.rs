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

//! Counted native trim boundaries without object or compiler authority.

use crate::native_tcl_utf::{NativeTclUtf, TclUtfUnit};
use std::ops::Range;
use tcl_dialect::TclVersion;

/// Default trim characters from the selected canonical release.
#[must_use]
pub fn default_trim_set(version: TclVersion) -> &'static [u8] {
    if version < TclVersion::V8_6 {
        b" \t\n\r"
    } else {
        b"\x09\x0a\x0b\x0c\x0d \xc0\x80\xc2\x85\xc2\xa0\xe1\x9a\x80\xe1\xa0\x8e\xe2\x80\x80\xe2\x80\x81\xe2\x80\x82\xe2\x80\x83\xe2\x80\x84\xe2\x80\x85\xe2\x80\x86\xe2\x80\x87\xe2\x80\x88\xe2\x80\x89\xe2\x80\x8a\xe2\x80\x8b\xe2\x80\xa8\xe2\x80\xa9\xe2\x80\xaf\xe2\x81\x9f\xe2\x81\xa0\xe3\x80\x80\xef\xbb\xbf"
    }
}

fn unit(policy: NativeTclUtf, version: TclVersion, bytes: &[u8]) -> Option<TclUtfUnit> {
    let mut unit = policy.decode_unit(bytes, None)?;
    if version == TclVersion::V8_6
        && (0xd800..=0xdbff).contains(&unit.value)
        && let Some(low) = policy.decode_unit(&bytes[unit.width..], Some(unit.value))
        && (0xdc00..=0xdfff).contains(&low.value)
    {
        unit.value = 0x10000 + ((unit.value - 0xd800) << 10) + (low.value - 0xdc00);
        unit.width += low.width;
    }
    Some(unit)
}

fn contains(
    policy: NativeTclUtf,
    version: TclVersion,
    characters: &[u8],
    value: u32,
) -> Option<bool> {
    let mut offset = 0;
    while offset < characters.len() {
        let next = unit(policy, version, &characters[offset..])?;
        if next.value == value {
            return Some(true);
        }
        offset += next.width;
    }
    Some(false)
}

fn right_unit(
    policy: NativeTclUtf,
    version: TclVersion,
    bytes: &[u8],
    end: usize,
) -> Option<(usize, TclUtfUnit)> {
    let mut start = policy.previous_character_boundary(bytes, end)?;
    if version == TclVersion::V8_6 {
        start = policy
            .previous_character_boundary(bytes, start)
            .unwrap_or(0);
    }
    loop {
        let next = unit(policy, version, &bytes[start..])?;
        if start + next.width >= end {
            return Some((start, next));
        }
        start += next.width;
    }
}

/// Byte range retained by the native left/right trim algorithms.
/// Original UTF byte boundaries and counted NUL are preserved; this function
/// neither performs getters nor constructs, copies or retains an object.
#[must_use]
pub fn trim_range(
    bytes: &[u8],
    characters: &[u8],
    version: TclVersion,
    left: bool,
    right: bool,
) -> Option<Range<usize>> {
    let policy = NativeTclUtf::for_version(version);
    let mut start = 0;
    let mut end = bytes.len();
    if characters.is_empty() {
        return Some(start..end);
    }
    if left {
        while start < end {
            let next = unit(policy, version, &bytes[start..])?;
            if !contains(policy, version, characters, next.value)? {
                break;
            }
            start += next.width;
        }
    }
    if right {
        while start < end {
            let (at, next) = right_unit(policy, version, &bytes[start..end], end - start)?;
            if !contains(policy, version, characters, next.value)? {
                if version <= TclVersion::V8_5 {
                    end = start + at + next.width;
                }
                break;
            }
            end = start + at;
        }
    }
    (start <= end && end <= bytes.len()).then_some(start..end)
}
