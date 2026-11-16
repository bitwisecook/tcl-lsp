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
fn original_mcp_event_reports_keep_requested_analysis_and_ignore_reporting_maps() {
    // Implementation contract: naming.consumer.original-mcp-event-source-advice
    // docs/design/analysis/name-resolution-proofs/original-mcp-event-source-advice.md
    let source = "when HTTP_REQUEST {}";
    let mut analysis = analyse(source, "f5-irules");
    analysis.all_defined_symbols.clear();
    analysis.global_scope.defined_symbols.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "counterfactual".to_owned();
    }
    assert_eq!(detect_events(source, &analysis)[0]["name"], "HTTP_REQUEST");
    assert_eq!(
        event_order_list(source, &analysis)[0]["name"],
        "HTTP_REQUEST"
    );
    assert!(detect_events(&format!("# moved\n{source}"), &analysis).is_empty());
    for profile in ["tcl8.6", "tcl9.0", "f5-iapps"] {
        let result = analyze(&json!({"source":source,"dialect":profile}));
        assert_eq!(result["events"], json!([]), "{profile}");
        assert_eq!(result["event_order"], json!([]), "{profile}");
    }
    let shadow = "proc when {args} {}; when HTTP_REQUEST {}";
    assert!(detect_events(shadow, &analyse(shadow, "f5-irules")).is_empty());
}
