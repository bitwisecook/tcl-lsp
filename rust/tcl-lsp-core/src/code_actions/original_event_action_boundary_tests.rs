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

// Implementation contract: naming.consumer.original-diagnostic-source-actions
// docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
#[test]
fn original_bootstrap_boundaries_exclude_nested_handler_lookalikes() {
    let source = "when HTTP_REQUEST {if {1} {when CLIENT_DATA {TCP::payload}}}\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
    let registry = analysis.resolved_registry().unwrap();
    let commands = diagnostic_context::current_commands(source, &analysis, registry).unwrap();
    let handlers = selected_event_handlers(source, &analysis, registry, &commands);
    assert_eq!(
        handlers.iter().map(|(_, event)| *event).collect::<Vec<_>>(),
        vec!["HTTP_REQUEST"]
    );
    assert_eq!(handlers[0].0.span.start(), 0);
}
