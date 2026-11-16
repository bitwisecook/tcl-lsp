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

//! Diagram data.

use super::*;

#[tokio::test]
async fn diagram_data_command_extracts_structured_flow() {
    let backend = test_backend();
    let src = "when HTTP_REQUEST {\n}\nwhen CLIENT_ACCEPTED {\n}\n";
    let out = backend
        .diagram_data_command(&[serde_json::json!(src)])
        .await
        .expect("some");
    let events: Vec<&str> = out
        .get("events")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|e| e.get("name").and_then(serde_json::Value::as_str))
        .collect();
    assert!(events.contains(&"HTTP_REQUEST"), "{events:?}");
    assert!(events.contains(&"CLIENT_ACCEPTED"), "{events:?}");
    assert!(
        out.get("procedures")
            .and_then(serde_json::Value::as_array)
            .is_some()
    );
    // No string argument → None (not an empty result).
    assert!(backend.diagram_data_command(&[]).await.is_none());
}
