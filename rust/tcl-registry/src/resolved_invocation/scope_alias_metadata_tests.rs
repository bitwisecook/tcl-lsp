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

use crate::CommandRegistry;
#[test]
fn selected_scope_alias_positions_keep_complete_roles_and_effective_ordinals() {
    // naming.analysis.selected-scope-alias-role-metadata
    // docs/design/analysis/name-resolution-proofs/selected-scope-alias-role-metadata.md
    // Selected authored descriptor positions only; no native link or frame.
    let registry = CommandRegistry::build_default();
    let query = Some(tcl_dialect::model::SurfaceQuery::core(
        tcl_dialect::model::Family::Tcl,
        "9.0",
    ));
    for (head, args, expected) in [
        ("global", vec!["g", "h"], vec![(0, None), (1, None)]),
        (
            "variable",
            vec!["name", "VALUE", "other"],
            vec![(0, None), (2, None)],
        ),
        ("upvar", vec!["1", "source", "local"], vec![(2, Some(1))]),
        (
            "namespace",
            vec!["upvar", "ns", "source", "local"],
            vec![(3, Some(2))],
        ),
        ("my", vec!["variable", "local"], vec![(1, None)]),
        ("puts", vec!["VALUE"], vec![]),
    ] {
        let selected = registry.resolve_invocation(head, &args, query).unwrap();
        let positions = selected.authored_source_scope_alias_arguments().unwrap();
        assert_eq!(
            positions
                .into_iter()
                .map(|position| (position.local, position.preceding_read))
                .collect::<Vec<_>>(),
            expected,
            "{head}"
        );
    }
}
