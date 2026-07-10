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

//! End-to-end LSP tests for the analyser / definition preview regressions:
//!
//! * #720 — `after 200 {…}` must not raise W001 (an integer time arg is not an
//!   unknown subcommand).
//! * #726 — `if {[myCmd [expr {1+1}]]}` must not raise W114 (the nested `[expr]`
//!   is a command argument, not a top-level expression context).
//! * #727 — go-to-definition of a `TclOO` method/constructor parameter must
//!   resolve to the parameter *name*, not the whole method body.
//! * #865 — a workspace file that was opened (and showed problems) must keep its
//!   Problems / File-Explorer badge after its editor tab closes: the server
//!   republishes the on-disk file's diagnostics rather than clearing them.
//!
//! Driven over real JSON-RPC against the `tower-lsp` service.

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

async fn read_until_id<R>(reader: &mut BufReader<R>, id: &str, max: usize) -> Option<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    for _ in 0..max {
        match tokio::time::timeout(Duration::from_secs(2), read_frame(reader)).await {
            Ok(body) if body.contains(id) => return Some(body),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}

async fn collect_frames<R>(reader: &mut BufReader<R>, window: Duration) -> Vec<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut frames = Vec::new();
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, read_frame(reader)).await {
            Ok(body) => frames.push(body),
            Err(_) => break,
        }
    }
    frames
}

/// Spawn a server, initialize a push-only client, return the reader + writer.
async fn start_session() -> (
    BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    tokio::io::WriteHalf<tokio::io::DuplexStream>,
    tokio::task::JoinHandle<()>,
) {
    let (client_side, server_side) = tokio::io::duplex(32768);
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
    let _ = read_until_id(&mut reader, "\"id\":1", 5).await;
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();
    (reader, client_write, server)
}

async fn did_open(
    writer: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>,
    uri: &str,
    text: &str,
) {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","languageId":"tcl","version":1,"text":"{escaped}"}}}}}}"#,
    );
    writer.write_all(frame(&msg).as_bytes()).await.unwrap();
}

async fn did_close(writer: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>, uri: &str) {
    let msg = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#,
    );
    writer.write_all(frame(&msg).as_bytes()).await.unwrap();
}

/// The published diagnostics frame for `uri` (or empty string if none seen).
async fn published_codes(
    reader: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    uri: &str,
) -> String {
    let frames = collect_frames(reader, Duration::from_millis(1200)).await;
    frames
        .into_iter()
        .rfind(|f| f.contains("publishDiagnostics") && f.contains(uri))
        .unwrap_or_default()
}

#[tokio::test]
async fn after_integer_ms_is_not_flagged_w001_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///after.tcl",
        "after 200 {puts \"Hello world!\"}\n",
    )
    .await;
    let diags = published_codes(&mut reader, "file:///after.tcl").await;
    assert!(
        !diags.contains("W001"),
        "after-integer must not raise W001 (#720): {diags}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn closed_on_disk_file_retains_diagnostics_badge_e2e() {
    // #865: opening a file surfaces its problems; closing the editor tab must NOT
    // wipe them — the file is still on disk and part of the workspace, so the
    // server republishes its on-disk diagnostics so the File-Explorer badge and
    // Problems entry survive the close.
    let dir = std::env::temp_dir().join(format!("tcl-lsp-865-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("retain.tcl");
    // `y` is set but never read → W211.
    let src = "proc foo {} { set y 1 }\n";
    std::fs::write(&path, src).unwrap();
    // A canonical file URI so the client's didOpen/didClose and the server's
    // on-disk re-read agree on the same path.
    let uri = format!("file://{}", path.display());

    let (mut reader, mut writer, server) = start_session().await;
    did_open(&mut writer, &uri, src).await;
    let opened = published_codes(&mut reader, &uri).await;
    assert!(
        opened.contains("W211"),
        "the open file should surface W211 first: {opened}",
    );

    // Close the tab; the file stays on disk.
    did_close(&mut writer, &uri).await;
    let after_close = published_codes(&mut reader, &uri).await;
    assert!(
        after_close.contains("publishDiagnostics") && after_close.contains("W211"),
        "#865: a closed on-disk file must retain its W211 badge, not be cleared: {after_close:?}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn nested_expr_in_command_sub_is_not_flagged_w114_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///w114.tcl",
        "proc myCmd {a} {return $a}\nif {[myCmd [expr {1 + 1}]]} {puts hi}\n",
    )
    .await;
    let diags = published_codes(&mut reader, "file:///w114.tcl").await;
    assert!(
        !diags.contains("W114"),
        "nested [expr] inside a command sub must not raise W114 (#726): {diags}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn method_param_definition_resolves_to_name_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    // Method `m {arg1 arg2}` with `$arg1` used in the body on line 2.
    did_open(
        &mut writer,
        "file:///oo.tcl",
        "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg1\n    }\n}\n",
    )
    .await;
    // Give diagnostics a moment, then request definition of `$arg1` (line 2,
    // char 15 — inside the `$arg1` usage).
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///oo.tcl"},"position":{"line":2,"character":15}}}"#;
    writer.write_all(frame(req).as_bytes()).await.unwrap();
    let resp = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("definition response");
    // The declaration is the parameter name on line 1 (the `method m {arg1 …}`
    // line), not the multi-line body. Assert the result range starts on line 1.
    assert!(
        resp.contains(r#""line":1"#),
        "param definition must point at the name on line 1, not the body (#727): {resp}",
    );
    // And it must NOT be the body span (which would start on line 1 col 25 and
    // end on a later line): the resolved range must be a single-line name span,
    // so the end line is also 1.
    assert!(
        !resp.contains(r#""line":3"#) && !resp.contains(r#""line":2,"character":13"#),
        "param definition must be a name-sized span, not the whole body (#727): {resp}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}
