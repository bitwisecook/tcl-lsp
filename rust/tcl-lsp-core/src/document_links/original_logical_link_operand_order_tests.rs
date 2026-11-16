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
fn logical_path_prefix_constants_require_static_earlier_written_operands() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = LinkContext {
        workspace_root: Some("/workspace"),
        ..LinkContext::default()
    };
    for source in [
        "set dir /ROOT; source [file join $dir leaf.tcl]",
        "set dir /ROOT; interp alias {} picksource {} source; picksource [file join $dir leaf.tcl]",
    ] {
        let links = document_links_in_context(source, profile, &context);
        assert_eq!(links.len(), 1, "{source}: {links:?}");
        assert_eq!(links[0].target, "file:///ROOT/leaf.tcl");
    }
    for source in [
        "set dir /ROOT; source -encoding [set dir /OTHER; list utf-8] [file join $dir leaf.tcl]",
        "set dir /ROOT; source -encoding [opaque] [file join $dir leaf.tcl]",
        "set dir /ROOT; source -encoding $encoding [file join $dir leaf.tcl]",
    ] {
        assert!(
            document_links_in_context(source, profile, &context).is_empty(),
            "{source}"
        );
    }
}
