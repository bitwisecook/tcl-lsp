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
fn actual_source_configuration_survives_frames_and_conflicting_join_is_unavailable() {
    let registry = CommandRegistry::build_default();
    let mut config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::default()
    };
    let context = ResolveContext {
        source_read_configuration: SourceReadConfiguration::Selected(config),
        ..ResolveContext::default()
    };
    let frame = context.in_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "actual-source-frame".to_owned(),
    });
    assert_eq!(frame.read_lexer_config(&registry), Some(config));
    let names = inner_read_places("${a{b}c}", &frame, &registry);
    assert!(names.iter().any(|place| place.name == "a{b"));
    assert!(!names.iter().any(|place| place.name == "a{b}c"));
    let mut alternate = frame.clone();
    config.braced_var = tcl_lexer::BracedVarStyle::Tcl9Nesting;
    alternate.source_read_configuration = SourceReadConfiguration::Selected(config);
    let mut joined = frame;
    joined.join(&alternate);
    assert_eq!(joined.read_lexer_config(&registry), None);
    assert_eq!(
        inner_read_places("$known", &joined, &registry),
        vec![place::unknown_top()]
    );
}
