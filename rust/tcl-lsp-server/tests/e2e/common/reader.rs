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

//! The background reader thread: framed messages routed to waiting requests, buffered, or answered automatically.

use super::*;

/// The background reader loop: parse framed messages from `stdout` and route
/// them. Server-initiated requests are answered via the shared stdin.
pub(super) fn read_loop(stdout: std::process::ChildStdout, shared: &Arc<Shared>) {
    let mut reader = BufReader::new(stdout);
    loop {
        // Read headers.
        let mut content_length = 0usize;
        let mut saw_header = false;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                // EOF or read error: server exited.
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            saw_header = true;
            if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        if !saw_header || content_length == 0 {
            continue;
        }
        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };
        route(&msg, shared);
    }
}

pub(super) fn route(msg: &Value, shared: &Arc<Shared>) {
    let has_id = msg.get("id").is_some_and(|v| !v.is_null());
    let is_request = msg.get("method").is_some();

    if has_id && !is_request {
        // Response to one of our requests.
        if let Some(id) = msg.get("id").and_then(Value::as_i64) {
            shared.responses.lock().unwrap().insert(id, msg.clone());
        }
    } else if has_id && is_request {
        // Server-initiated request — record it (so a test can assert the
        // server actually asked), then answer so the server never blocks.
        shared.server_requests.lock().unwrap().push(msg.clone());
        shared.requests_cv.notify_all();
        auto_reply(msg, shared);
    } else {
        // Notification.
        shared.notifications.lock().unwrap().push(msg.clone());
        shared.notify_cv.notify_all();
    }
}

pub(super) fn auto_reply(msg: &Value, shared: &Arc<Shared>) {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let result = if method == "workspace/configuration" {
        let delay = *shared.configuration_reply_delay.lock().unwrap();
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
        let items = msg
            .get("params")
            .and_then(|p| p.get("items"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Value::Array(
            items
                .iter()
                .map(|item| {
                    if item.get("section").and_then(Value::as_str) != Some("tclLsp") {
                        return Value::Null;
                    }
                    let scoped = item
                        .get("scopeUri")
                        .and_then(Value::as_str)
                        .and_then(|scope| {
                            shared
                                .folder_configs
                                .lock()
                                .unwrap()
                                .get(scope.trim_end_matches('/'))
                                .cloned()
                        });
                    scoped.unwrap_or_else(|| shared.tcllsp_config.lock().unwrap().clone())
                })
                .collect(),
        )
    } else if method == "window/showMessageRequest" {
        shared
            .message_action_reply
            .lock()
            .unwrap()
            .as_ref()
            .map_or(Value::Null, |title| json!({ "title": title }))
    } else if method == "window/showDocument" {
        json!({ "success": true })
    } else {
        Value::Null
    };
    let payload = json!({ "jsonrpc": "2.0", "id": msg.get("id"), "result": result });
    let body = serde_json::to_vec(&payload).unwrap_or_default();
    let mut stdin = shared.stdin.lock().unwrap();
    let _ = write!(stdin, "Content-Length: {}\r\n\r\n", body.len());
    let _ = stdin.write_all(&body);
    let _ = stdin.flush();
}
