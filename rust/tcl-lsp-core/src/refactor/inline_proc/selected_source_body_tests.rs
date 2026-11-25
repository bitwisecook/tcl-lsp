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
fn logical_inline_call_selection_uses_current_original_body_regions() {
    // naming.source.original-refactor-command-selection
    // docs/design/analysis/name-resolution-proofs/original-refactor-command-selection.md
    let source = "interp alias {} run {} eval\nproc p {x} {expr {$x + 1}}\nrun {p 2}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let cursor = u32::try_from(source.rfind("p 2").unwrap()).unwrap();
    let command = super::super::find_original_command_at(source, cursor, &analysis).unwrap();
    assert_eq!(command.name(), "p");
    assert_eq!(command.span.start(), cursor);
    let registry = analysis.resolved_registry().unwrap();
    let action = inline_proc(source, cursor, &analysis, registry).unwrap();
    assert!(action.disabled.is_none(), "{:?}", action.disabled);
    assert!(
        action
            .try_apply(source)
            .unwrap()
            .ends_with("run {expr {2 + 1}}")
    );
    let inert = "proc eval args {}; proc p {x} {expr {$x + 1}}; eval {p 2}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(inert, "tcl");
    let cursor = u32::try_from(inert.rfind("p 2").unwrap()).unwrap();
    let command = super::super::find_original_command_at(inert, cursor, &analysis).unwrap();
    assert_ne!(command.span.start(), cursor);
    assert_eq!(command.name(), "eval");
}

#[test]
fn original_logical_inline_keeps_selected_expression_and_refuses_frame_sensitive_alias() {
    // naming.refactor.original-frame-traversal
    // docs/design/analysis/name-resolution-proofs/refactor-original-frame-traversal.md
    let source = "proc p {x} {expr {$x + 1}}
p 2";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let registry = CommandRegistry::build_default();
    let cursor = u32::try_from(source.rfind("p 2").unwrap()).unwrap();
    let action = inline_proc(source, cursor, &analysis, &registry).unwrap();
    assert!(action.disabled.is_none(), "{:?}", action.disabled);
    assert!(action.try_apply(source).unwrap().ends_with("expr {2 + 1}"));
    analysis.resolved_input = None;
    assert!(inline_proc(source, cursor, &analysis, &registry).is_none());
    let source = "interp alias {} finish {} return
proc p {} {finish 1}
p";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
    let proc_def = analysis.all_procs.get("::p").unwrap();
    let body = single_command_body(source, proc_def, &analysis, walk.config).unwrap();
    assert!(reject_frame_sensitive_body(source, &body, &[], &walk).is_err());
}
