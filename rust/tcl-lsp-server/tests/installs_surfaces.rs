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

//! The binary installs its command surfaces before its first registry read.
//! This harness installs nothing itself, so a binary that read the registry
//! first would panic with the registry's not-installed message.

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

fn frame(body: &str) -> Vec<u8> {
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

/// Read one framed message, or `None` at end of stream.
fn read_message(reader: &mut impl BufRead) -> Option<String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            length = value.parse().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

fn send(stdin: &Mutex<ChildStdin>, body: &str) {
    let mut stdin = stdin.lock().expect("the stdin lock");
    let _ = stdin.write_all(&frame(body));
    let _ = stdin.flush();
}

/// Answer a request the server sends the client, so it never waits on one:
/// `workspace/configuration` with no settings for each item, anything else
/// with `null`.
fn answer(stdin: &Mutex<ChildStdin>, request: &serde_json::Value) {
    let result = if request["method"] == "workspace/configuration" {
        let items = request["params"]["items"].as_array().map_or(0, Vec::len);
        serde_json::Value::Array(vec![serde_json::Value::Null; items])
    } else {
        serde_json::Value::Null
    };
    let reply = serde_json::json!({"jsonrpc": "2.0", "id": request["id"], "result": result});
    send(stdin, &reply.to_string());
}

#[test]
fn the_server_installs_its_surfaces_before_analysing_a_document() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tcl-lsp-server"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the server");
    let stdin = Arc::new(Mutex::new(child.stdin.take().expect("the server's stdin")));
    send(
        &stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#,
    );
    let mut stdout = BufReader::new(child.stdout.take().expect("the server's stdout"));
    let (sender, receiver) = mpsc::channel();
    let replies = Arc::clone(&stdin);
    std::thread::spawn(move || {
        while let Some(message) = read_message(&mut stdout) {
            let Ok(message) = serde_json::from_str::<serde_json::Value>(&message) else {
                continue;
            };
            if message.get("method").is_some() && message.get("id").is_some() {
                answer(&replies, &message);
            } else if message["id"] == 1 {
                let _ = sender.send("initialized");
            } else if message["method"] == "textDocument/publishDiagnostics" {
                let _ = sender.send("published");
            }
        }
    });
    let deadline = Duration::from_secs(120);
    let initialised = receiver.recv_timeout(deadline) == Ok("initialized");
    // A notification sent before the initialize answer is dropped.
    for body in [
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///probe.tcl","languageId":"tcl","version":1,"text":"set x [llength {a b}]\nputs $x\n"}}}"#,
    ] {
        send(&stdin, body);
    }
    let published = initialised && receiver.recv_timeout(deadline) == Ok("published");
    for body in [
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#,
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#,
    ] {
        send(&stdin, body);
    }
    let output = child.wait_with_output().expect("wait for the server");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("no command surface set is installed"),
        "{stderr}"
    );
    assert_ne!(
        output.status.code(),
        Some(101),
        "the binary panicked: {stderr}"
    );
    assert!(
        output.status.code().is_some(),
        "the binary was killed: {stderr}"
    );
    assert!(published, "the server published diagnostics: {stderr}");
}
