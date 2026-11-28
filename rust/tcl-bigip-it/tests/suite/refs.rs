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

use tcl_bigip::refs::references_at;
const CONF: &str = "ltm pool /Common/p {\n    members {\n        /Common/n:80 { }\n    }\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";
fn col_of(source: &str, line: u32, needle: &str) -> u32 {
    u32::try_from(
        source
            .lines()
            .nth(line as usize)
            .unwrap()
            .find(needle)
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn references_excludes_declaration_when_requested() {
    crate::install();
    let line = 6;
    let col = col_of(CONF, line, "/Common/p");
    let refs = references_at(CONF, line, col, false);
    let lines: Vec<u32> = refs.iter().map(|r| r.start.line).collect();
    assert!(
        !lines.contains(&0),
        "declaration (line 0) must be dropped: {lines:?}",
    );
    assert!(
        lines.contains(&6),
        "reference (line 6) must remain: {lines:?}"
    );
}
