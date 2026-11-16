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
fn constructed_class_graph_lookup_preserves_literal_colon_holder() {
    // Implementation contract: naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Explicit constructed graph assistance; no retained source lookup exists here.
    let tails: HashMap<String, Vec<String>> = HashMap::new();
    for (owner, expected) in [
        ("Sub", "::Base"),
        ("::Sub", "::Base"),
        ("n::Sub", "::n::Base"),
        ("::a::::Sub", "::a::Base"),
        ("::::::", ":::::Base"),
        ("::é::λ", "::é::Base"),
    ] {
        assert_eq!(
            resolve_class_name("Base", owner, |candidate| candidate == expected, &tails).as_deref(),
            Some(expected),
        );
        assert_eq!(
            resolve_class_name(
                "Base",
                owner,
                |candidate| candidate == "::foreign::Base",
                &tails
            ),
            None,
        );
    }
}
