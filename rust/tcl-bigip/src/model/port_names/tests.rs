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

#[test]
fn name_to_port_looks_up_well_known_services() {
    // mcpd SCF service-name → port table; unknown names return None.
    assert_eq!(name_to_port("http"), Some(80));
    assert_eq!(name_to_port("https"), Some(443));
    assert_eq!(name_to_port("ssh"), Some(22));
    assert_eq!(name_to_port("ftp"), Some(21));
    assert_eq!(name_to_port("smtp"), Some(25));
    assert_eq!(name_to_port("domain"), Some(53));
    assert_eq!(name_to_port("not-a-real-service-xyz"), None);
}

#[test]
fn name_to_port_table_is_sorted_for_binary_search() {
    // `name_to_port` binary-searches each section, so the keys must be
    // sorted ascending within a section and from one section to the next.
    let keys: Vec<&str> = SECTIONS
        .iter()
        .flat_map(|section| section.iter().map(|(key, _)| *key))
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
}
