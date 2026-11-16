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
fn original_loop_body_and_step_parsers_keep_bom_as_command_content() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Retained conditional schema/body syntax; no executed loop/cell effect proof.
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl9.1").unwrap();
    let metadata = BoundsMetadataContext::Retained(input.borrowed_context_registry());
    assert_eq!(
        body_increment("incr counter", "counter", metadata, input.lexer_config()),
        Some(1)
    );
    assert_eq!(
        body_increment(
            "\u{feff}incr counter",
            "counter",
            metadata,
            input.lexer_config()
        ),
        None
    );
    assert_eq!(
        sole_command("incr counter", input.lexer_config())
            .unwrap()
            .texts[0],
        "incr"
    );
    assert_eq!(
        sole_command("\u{feff}incr counter", input.lexer_config())
            .unwrap()
            .texts[0],
        "\u{feff}incr"
    );
    assert!(sole_command("incr counter; incr counter", input.lexer_config()).is_none());
}

#[test]
fn original_body_walk_keeps_leading_bom_as_command_content_at_each_depth() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Retained available schema plus lexical body topology, no entered loop/exit.
    let context = tcl_registry::model::ingress::static_context_for("tcl9.1");
    let config = tcl_lexer::LexerConfig::for_file_grammar(
        crate::environment_ingress::resolve_environment("tcl9.1").grammar(),
    );
    let metadata = BoundsMetadataContext::Retained(context);
    assert!(body_may_exit("break", metadata, config));
    assert!(!body_may_exit("\u{feff}break", metadata, config));
    assert!(!body_may_exit("if 1 {\u{feff}break}", metadata, config));
    assert!(body_may_exit("if 1 {break}", metadata, config));
}
