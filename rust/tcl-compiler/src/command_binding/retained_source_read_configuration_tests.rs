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
fn source_entry_and_baseline_rebuild_keep_actual_full_lexer_config() {
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig {
        strict_quoting: true,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::default()
    };
    let selected = ModuleCommandBindings::initial_with_options(
        &registry,
        SourceAnalysisOptions::default(),
        Some(config),
    );
    assert_eq!(
        selected.source_variables.read_lexer_config(&registry),
        Some(config)
    );
    let rebuilt = ModuleCommandBindings::initial_entry_from_baseline(
        &registry,
        Arc::clone(&selected.baseline),
    );
    assert_eq!(
        rebuilt.source_variables.read_lexer_config(&registry),
        Some(config)
    );
    let mut changed = config;
    changed.strict_quoting = false;
    let alternate = ModuleCommandBindings::initial_with_options(
        &registry,
        SourceAnalysisOptions::default(),
        Some(changed),
    );
    assert_ne!(selected.baseline, alternate.baseline);
}
