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

//! Encoding of object-name segments at the iControl REST boundary.

use tcl_bigip::object_names::BigIpName;

/// Convert `/Common/foo` -> `~Common~foo` (iControl REST format).
#[must_use]
pub fn encode_path(full_path: &str) -> String {
    if full_path.starts_with('/') {
        percent_encode(&full_path.replace('/', "~"))
    } else {
        // Percent-encode everything but the RFC 3986 unreserved set.
        percent_encode(full_path)
    }
}

/// Percent-encode every byte that is not an unreserved character
/// (`A-Z a-z 0-9 _ . - ~`), encoding all others including `/`.
fn percent_encode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Decode an iControl REST resource ID into an authored path or short name.
///
/// Returns `None` for an invalid escape, UTF-8 sequence or object path.
#[must_use]
pub fn decode_path(resource_id: &str) -> Option<String> {
    let Some(segments) = resource_id.strip_prefix('~') else {
        return decode_component(resource_id);
    };
    let mut path = String::with_capacity(resource_id.len());
    for segment in segments.split('~') {
        path.push('/');
        path.push_str(&decode_component(segment)?);
    }
    BigIpName::try_parse(&path)?;
    Some(path)
}

fn decode_component(text: &str) -> Option<String> {
    let mut input = text.as_bytes().iter().copied();
    let mut decoded = Vec::with_capacity(text.len());
    while let Some(byte) = input.next() {
        let byte = if byte == b'%' {
            let digits = [input.next()?, input.next()?];
            if !digits.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            u8::from_str_radix(std::str::from_utf8(&digits).ok()?, 16).ok()?
        } else {
            byte
        };
        if byte == b'/' || byte.is_ascii_control() {
            return None;
        }
        decoded.push(byte);
    }
    let decoded = String::from_utf8(decoded).ok()?;
    (!decoded.is_empty() && decoded.trim() == decoded).then_some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_resource_id_round_trips_route_domains_and_folder_separators() {
        for (name, encoded) in [
            ("/Common/10.0.0.1%2", "~Common~10.0.0.1%252"),
            (
                "/Tenant/app.app/2001:db8::1",
                "~Tenant~app.app~2001%3Adb8%3A%3A1",
            ),
            ("short", "short"),
        ] {
            assert_eq!(encode_path(name), encoded);
            assert_eq!(decode_path(encoded).as_deref(), Some(name));
        }
    }

    #[test]
    fn rest_resource_id_rejects_invalid_escapes_and_paths() {
        for input in [
            "~Common~na%ME",
            "~Common~~name",
            "~Common~",
            "~Common~f%2Fname",
        ] {
            assert_eq!(decode_path(input), None, "{input}");
        }
    }
}
