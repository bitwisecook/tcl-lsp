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

#[test]
fn original_recovery_scan_and_gate_share_the_supplied_whole_configuration() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Conditional signature candidates; no command installation or entry.
    let registry = tcl_registry::CommandRegistry::build_default();
    let extra = std::sync::Arc::default();
    let source = "proc hidden {} {}; set x ${a{}";
    let config = tcl_lexer::LexerConfig::default();
    let first_close = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..config
    };
    let complete = super::recovery_known_commands(source, &registry, &extra, first_close);
    assert!(!complete.contains("hidden"));
    let incomplete = super::recovery_known_commands(source, &registry, &extra, config);
    assert!(incomplete.contains("hidden"));
    assert!(complete.contains("set"));
    assert!(incomplete.contains("set"));
}
