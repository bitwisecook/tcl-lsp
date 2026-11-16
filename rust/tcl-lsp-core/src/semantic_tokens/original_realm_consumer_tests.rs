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
use tcl_compiler::analyser::Analyser;

#[test]
fn original_semantic_consumer_declines_missing_realm_without_nominal_recapture() {
    // naming.core.original-command-source-schema
    // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
    let source = "set value 1; proc p {} {return $value}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let profile = analysis.resolved_profile().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let entries = collect_entries(
        source,
        profile,
        registry,
        None,
        WorkspaceTokenFacts {
            analysis: Some(&analysis),
            ..WorkspaceTokenFacts::default()
        },
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.3 == TokenKind::Variable && entry.4 & MOD_DECLARATION != 0)
    );
    let mut unowned = AnalysisResult::default();
    unowned.resolved_input = analysis.resolved_input.clone();
    unowned.body_lexer_config = analysis.body_lexer_config;
    unowned.all_procs.clone_from(&analysis.all_procs);
    unowned.all_variables.clone_from(&analysis.all_variables);
    assert!(unowned.retained_command_realm().is_none());
    assert_eq!(
        collect_entries(
            source,
            profile,
            registry,
            None,
            WorkspaceTokenFacts {
                analysis: Some(&unowned),
                ..WorkspaceTokenFacts::default()
            }
        ),
        [] as [(u32, u32, u32, crate::semantic_tokens::TokenKind, u32); 0]
    );
}
