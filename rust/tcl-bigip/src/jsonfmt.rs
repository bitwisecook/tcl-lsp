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

//! JSON string output for the byte-stable graph, stats, and cleanup reports.

/// A double-quoted JSON string, escaping non-ASCII and preserving ASCII DEL.
#[must_use]
pub fn json_string(s: &str) -> String {
    use std::fmt::Write as _;

    let json = serde_json::to_string(s).expect("serialise JSON string");
    if s.is_ascii() {
        return json;
    }
    let mut ascii = String::with_capacity(json.len());
    for ch in json.chars() {
        if ch.is_ascii() {
            ascii.push(ch);
        } else {
            for unit in ch.encode_utf16(&mut [0; 2]) {
                let _ = write!(ascii, "\\u{unit:04x}");
            }
        }
    }
    ascii
}
