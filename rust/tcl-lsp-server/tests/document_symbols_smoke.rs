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

//! End-to-end LSP smoke test for the document-symbol provider.
//!
//! Drives the server through `tower_lsp_server::LspService` over an
//! in-memory duplex pipe so the test exercises the same
//! `initialize` → `textDocument/didOpen` →
//! `textDocument/documentSymbol` sequence a real client would.
//! Verifies that a `proc` definition shows up in the response as a
//! `Function`-kind symbol.

use std::time::Duration;

use tcl_lsp_server::Backend;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tower_lsp_server::{LspService, Server};

fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

async fn read_frame<R>(reader: &mut BufReader<R>) -> String
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut header = String::new();
    let mut content_length = 0usize;
    loop {
        header.clear();
        let n = reader
            .read_line(&mut header)
            .await
            .expect("reading LSP header");
        assert!(n > 0, "unexpected EOF reading LSP header");
        let trimmed = header.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().expect("parsing Content-Length");
        }
    }
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .expect("reading LSP body");
    String::from_utf8(body).expect("UTF-8 LSP body")
}

#[tokio::test]
async fn document_symbol_smoke() {
    let (client_side, server_side) = tokio::io::duplex(8192);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, mut client_write) = tokio::io::split(client_side);

    let (service, socket) = LspService::new(Backend::new);
    let server = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });

    let mut reader = BufReader::new(client_read);

    // 1. initialize
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let resp = read_frame(&mut reader).await;
    assert!(resp.contains("\"id\":1"), "initialize response: {resp}");
    assert!(
        resp.contains("\"documentSymbolProvider\""),
        "expected documentSymbolProvider capability: {resp}",
    );

    // 2. initialized notification
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;

    // 3. textDocument/didOpen with a single-proc file.
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///t.tcl","languageId":"tcl","version":1,"#,
        r#""text":"proc demo {a b} {\n  set x 1\n  return $x\n}\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // 4. textDocument/documentSymbol request.
    let sym_req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///t.tcl"}}}"#;
    client_write
        .write_all(frame(sym_req).as_bytes())
        .await
        .unwrap();

    // Drain frames until we see id=2 (any intervening log messages
    // get tossed).
    let mut sym_resp = String::new();
    // Scan to the id=2 response under a deadline, not a fixed frame count:
    // the server interleaves log and diagnostics notifications on the same
    // channel, and how many arrive first is not a contract.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        let Ok(frame_body) = tokio::time::timeout_at(deadline, read_frame(&mut reader)).await
        else {
            break;
        };
        if frame_body.contains("\"id\":2") {
            sym_resp = frame_body;
            break;
        }
    }
    assert!(
        !sym_resp.is_empty(),
        "did not receive id=2 documentSymbol response within the deadline",
    );
    assert!(
        sym_resp.contains("\"name\":\"demo\""),
        "expected `demo` symbol in response, got {sym_resp}",
    );
    // SymbolKind::FUNCTION = 12 in the LSP spec.
    assert!(
        sym_resp.contains("\"kind\":12"),
        "expected Function kind (12) in response, got {sym_resp}",
    );

    // 5. shutdown so the server task can exit cleanly.
    let shutdown = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#;
    client_write
        .write_all(frame(shutdown).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;

    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);

    server.abort();
}

/// `append` / `lappend` create their target variable, so it must surface as a
/// `Variable`-kind document symbol — the same as `set`. Guards the fix
/// that made the analyser record `append`/`lappend` targets.
#[tokio::test]
async fn document_symbol_includes_append_lappend_vars() {
    let (client_side, server_side) = tokio::io::duplex(8192);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, mut client_write) = tokio::io::split(client_side);

    let (service, socket) = LspService::new(Backend::new);
    let server = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    let mut reader = BufReader::new(client_read);

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let _ = read_frame(&mut reader).await;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;

    // Top-level `lappend` / `append` targets are global variables.
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///vars.tcl","languageId":"tcl","version":1,"#,
        r#""text":"lappend safe 1\nappend note hi\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    let sym_req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///vars.tcl"}}}"#;
    client_write
        .write_all(frame(sym_req).as_bytes())
        .await
        .unwrap();

    let mut sym_resp = String::new();
    // Scan to the id=2 response under a deadline, not a fixed frame count:
    // the server interleaves log and diagnostics notifications on the same
    // channel, and how many arrive first is not a contract.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        let Ok(frame_body) = tokio::time::timeout_at(deadline, read_frame(&mut reader)).await
        else {
            break;
        };
        if frame_body.contains("\"id\":2") {
            sym_resp = frame_body;
            break;
        }
    }
    assert!(!sym_resp.is_empty(), "no id=2 documentSymbol response");
    assert!(
        sym_resp.contains("\"name\":\"safe\""),
        "expected `safe` (lappend target) symbol, got {sym_resp}",
    );
    assert!(
        sym_resp.contains("\"name\":\"note\""),
        "expected `note` (append target) symbol, got {sym_resp}",
    );
    // SymbolKind::VARIABLE = 13 in the LSP spec.
    assert!(
        sym_resp.contains("\"kind\":13"),
        "expected Variable kind (13) in response, got {sym_resp}",
    );

    let shutdown = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#;
    client_write
        .write_all(frame(shutdown).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    server.abort();
}
