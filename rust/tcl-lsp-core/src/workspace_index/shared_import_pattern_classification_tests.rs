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
fn escaped_import_patterns_retain_original_names_in_each_supported_oracle() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (name, pattern) in [
            ("p*", r"p\*"),
            ("p?", r"p\?"),
            ("p[", r"p\["),
            (r"p\x", r"p\\x"),
        ] {
            let source = [
                    format!("namespace eval ::Source {{proc {{{name}}} {{}} {{return OK}}; namespace export *}}"),
                    format!("namespace eval ::Target {{namespace import {{::Source::{pattern}}}; {{{name}}}}}"),
                ].join("\n");
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, dialect);
            let mut index = WorkspaceIndex::new();
            index.add_document("original", &analysis);
            let candidates = vec![
                tcl_syntax::naming::qualify("::Target", name),
                tcl_syntax::naming::qualify("::", name),
            ];
            let expected = tcl_syntax::naming::qualify("::Source", name);
            let actual = index.resolve_wildcard_import(
                name,
                &candidates,
                CallSite {
                    uri: "original",
                    at: u32::try_from(source.len() - 1).unwrap(),
                    enclosing_body: None,
                },
            );
            assert_eq!(
                actual.as_deref(),
                Some(expected.as_str()),
                "{dialect}/{pattern}"
            );
        }
    }
}
