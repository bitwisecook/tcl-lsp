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

//! Dump the BIG-IP object registry as JSONL — one line per spec with its
//! header types and property names. Used by the registry differential test
//! to assert the registry carries every property the golden registry declares.
//!
//! Usage: `cargo run -p tcl-registry --example dump_bigip_registry`

use std::fmt::Write as _;

fn main() {
    let reg = tcl_registry::bigip::BigipRegistry::build();
    let mut out = String::new();
    for spec in reg.specs() {
        let headers: Vec<String> = spec
            .header_types
            .iter()
            .map(|(m, o)| format!("{m}|{o}"))
            .collect();
        let props: Vec<&str> = spec.properties.iter().map(|p| p.name).collect();
        // Minimal JSON, no external serialiser dependency.
        let h = headers
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let p = props
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(out, "{{\"headers\":[{h}],\"props\":[{p}]}}");
    }
    print!("{out}");
}
