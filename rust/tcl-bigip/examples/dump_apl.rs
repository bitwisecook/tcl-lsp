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

//! Parse an F5 iApp APL file and print its canonical JSON document.
//!
//! Used by the differential harness: `dump_apl <path>` prints the
//! JSON that `_rust_bridge.rebuild_apl` reconstructs the
//! `AplModel` from.

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_apl <path>");
    let src = std::fs::read_to_string(&path).expect("read source");
    let model = tcl_bigip::apl::parse_apl(&src);
    let json = tcl_bigip::apl::model_to_canonical(&model);
    println!("{}", serde_json::to_string(&json).expect("serialise"));
}
