// tcl-lsp -- a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end coverage for the handler/input liveness cycle in issue #1345.

use std::time::Duration;

use serde_json::json;

use super::common::Lsp;

/// Four workspace notifications each enter a server-to-client configuration
/// round trip. While their replies are delayed, enough ordinary requests are
/// sent to exceed `tower-lsp-server`'s 100-item task channel. The final request
/// can complete only if stdin continues past the burst and routes the delayed
/// client replies which release those four handlers.
#[test]
fn delayed_client_replies_are_not_stranded_behind_the_handler_queue() {
    let mut lsp = Lsp::tcl();
    lsp.set_configuration_reply_delay(Duration::from_millis(100));

    for _ in 0..4 {
        lsp.notify(
            "workspace/didChangeWorkspaceFolders",
            json!({ "event": { "added": [], "removed": [] } }),
        );
    }
    for _ in 0..400 {
        lsp.send_request_no_wait("textDocument/hover", json!({}));
    }
    let final_id = lsp.send_request_no_wait(
        "workspace/executeCommand",
        json!({
            "command": "tcl-lsp.getEffectiveConfig",
            "arguments": [""]
        }),
    );

    let response = lsp.await_response(
        final_id,
        "tcl-lsp.getEffectiveConfig",
        Duration::from_secs(5),
    );
    assert!(
        response.get("result").is_some(),
        "server did not recover after delayed client replies: {response}"
    );
}
