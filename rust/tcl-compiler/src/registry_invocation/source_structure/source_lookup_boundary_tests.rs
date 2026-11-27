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
fn original_source_lookup_facet_is_independent_of_advisory_body_roles() {
    // Implementation contract: naming.source.closed-syntax-script-regions
    // docs/design/analysis/name-resolution-proofs/closed-syntax-script-regions.md
    for (source, preserved) in [
        ("if {1} {puts quiet}\n", true),
        ("if [rename if saved] {puts data}\n", false),
        // #2427: an autoload index entry keeps `unavailable` unbounded.
        (
            "set ::auto_index(unavailable) {source lib.tcl}\nunavailable\nif {1} {puts advisory}\n",
            false,
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, commands.last().unwrap()).unwrap();
        assert_eq!(
            words.roles(),
            Some(
                [
                    (0, tcl_registry::ArgRole::Expr),
                    (1, tcl_registry::ArgRole::Body)
                ]
                .as_slice()
            ),
            "ordinary unknown expression payload preserves source positions: {source}"
        );
        if source.contains("[rename") {
            assert!(matches!(
                words.arguments()[0],
                crate::registry_invocation::EffectiveInvocationWord::Dynamic
            ));
        }
        assert_eq!(
            words.operands_preserve_source_lookup(),
            preserved,
            "{source}"
        );
    }
}
