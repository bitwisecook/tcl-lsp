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

use crate::analyser::Analyser;

#[test]
fn original_namespace_path_retains_all_native_children_after_quoted_escape_rendering() {
    let source = r#"namespace eval ::first {}; namespace path "::first ::opaque\uD800 ::third""#;
    let result = Analyser::new().analyse(source, "tcl8.6");
    let values = result
        .namespace_refs
        .iter()
        .filter_map(|reference| reference.original_name_input.as_ref())
        .filter(|input| {
            input
                .original_static_list_container()
                .is_some_and(|container| !container.ordinals().is_empty())
        })
        .map(|input| input.bytes().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        [
            b"::first".to_vec(),
            b"::opaque\xed\xa0\x80".to_vec(),
            b"::third".to_vec()
        ]
    );
}
