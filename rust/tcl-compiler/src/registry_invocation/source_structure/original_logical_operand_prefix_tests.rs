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
fn logical_operand_prefix_retains_only_static_earlier_written_words() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for (source, argument, expected_prefix) in [
        ("set dir /ROOT; source [file join $dir leaf.tcl]", 0, 1),
        (
            "set dir /ROOT; source -encoding utf-8 [file join $dir leaf.tcl]",
            2,
            3,
        ),
        (
            "set dir /ROOT; interp alias {} picksource {} source -encoding utf-8; picksource [file join $dir leaf.tcl]",
            2,
            1,
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile.name);
        let context = analysis
            .resolved_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry();
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, commands.last().unwrap()).unwrap();
        let receipt = words
            .original_logical_static_operand_prefix_before(context, argument)
            .unwrap();
        assert_eq!(receipt.original_invocation(), &words);
        assert_eq!(receipt.argument(), argument);
        assert_eq!(receipt.original_written_prefix().len(), expected_prefix);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        assert!(
            words
                .original_logical_static_operand_prefix_before(&foreign, argument)
                .is_none()
        );
    }
    for source in [
        "set dir /ROOT; source -encoding [set dir /OTHER; list utf-8] [file join $dir leaf.tcl]",
        "set dir /ROOT; source -encoding [opaque] [file join $dir leaf.tcl]",
        "set dir /ROOT; source -encoding $encoding [file join $dir leaf.tcl]",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile.name);
        let context = analysis
            .resolved_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry();
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = source_registry_words(source, &analysis, commands.last().unwrap())
            .expect("actual conditional source schema remains independent of operand continuity");
        assert!(
            words
                .original_logical_static_operand_prefix_before(context, 2)
                .is_none(),
            "{source}"
        );
    }
}
