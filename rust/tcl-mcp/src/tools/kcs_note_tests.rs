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

const BEGIN: &str = "<!-- BEGIN GENERATED: tcl-mcp tool table (TCL_MCP_BLESS=1 cargo test -p tcl-mcp kcs_note) -->\n";
const END: &str = "<!-- END GENERATED: tcl-mcp tool table -->";

#[test]
fn kcs_note_tool_table_matches_the_catalogue() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/kcs/features/kcs-feature-mcp-server.md");
    let note = std::fs::read_to_string(&path).expect("the KCS note");
    let start = note.find(BEGIN).expect("the generated-table begin marker") + BEGIN.len();
    let end = note.find(END).expect("the generated-table end marker");
    let table = super::kcs_tool_table();
    if note[start..end] == table {
        return;
    }
    if std::env::var_os("TCL_MCP_BLESS").is_some_and(|v| v == "1") {
        std::fs::write(&path, format!("{}{table}{}", &note[..start], &note[end..]))
            .expect("rewrite the KCS note");
        return;
    }
    panic!(
        "{} is stale against the tool catalogue; run `TCL_MCP_BLESS=1 cargo test -p tcl-mcp kcs_note`",
        path.display()
    );
}

#[test]
fn every_tool_has_a_row() {
    let table = super::kcs_tool_table();
    for tool in super::TOOLS {
        assert!(
            table.contains(&format!("| `{}` |", tool.name)),
            "{}",
            tool.name
        );
    }
    assert!(
        table.contains("`tier` (bundled, user, workspace, studio-override)"),
        "{table}"
    );
}
