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
fn braced_head_split_uses_supplied_axis_and_reuse_retires_inventory() {
    let mut analyser = Analyser::new();
    let mut input = analyser.prepare_analysis_input("tcl9.0").unwrap();
    input.config.braced_var = tcl_dialect::BracedVarStyle::FirstClose;
    let mut analyser = analyser.with_resolved_input(input.clone());
    assert_eq!(
        analyser.split_braced_head("a{b}c}::tail"),
        ("a{b", "c}::tail")
    );
    assert!(!analyser.builtin_command_names().is_empty());
    assert!(analyser.builtin_names.is_some());
    let mut analyser = analyser.with_resolved_input(input);
    assert!(analyser.builtin_names.is_none());
    assert!(!analyser.builtin_command_names().is_empty());
    analyser.clear_run_state();
    assert!(analyser.builtin_names.is_none());
    assert!(analyser.builtin_dialect.is_none());
    let mut analyser = Analyser::new();
    analyser.prepare_analysis_input("tcl9.0").unwrap();
    assert!(!analyser.builtin_command_names().is_empty());
    let analyser = analyser.with_pack_overlay(u64::MAX);
    assert!(analyser.builtin_names.is_none());
    assert!(analyser.builtin_dialect.is_none());
}
