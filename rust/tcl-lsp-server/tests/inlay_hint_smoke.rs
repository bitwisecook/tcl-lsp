//! End-to-end LSP smoke test for `tcl-lsp-server` inlay hints.
//!
//! Drives the server through `tower_lsp_server::LspService` over an in-memory
//! duplex pipe and verifies the **default-off** contract: inlay hints are
//! opt-in, so a request against a document with hintable content (a proc
//! definition plus a call site) still answers with a well-formed empty
//! list — never an error or `null` — until the `inlayParameterHints`
//! feature is explicitly enabled.

use std::time::Duration;

use tcl_lsp_server::Backend;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tower_lsp_server::{LspService, Server};

/// Frame `body` as an LSP `Content-Length: …\r\n\r\n<body>` packet.
fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Read one LSP-framed JSON message from `reader`, returning the body.
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
            break; // header / body separator
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
async fn inlay_hints_default_off_returns_empty_list() {
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

    // 1. initialize (no feature overrides → inlay hints default off).
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let resp = read_frame(&mut reader).await;
    assert!(resp.contains("\"id\":1"), "initialize response: {resp}");
    assert!(
        resp.contains("\"inlayHintProvider\""),
        "expected inlayHintProvider capability: {resp}",
    );

    // 2. initialized notification (drain the server's log reply).
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;

    // 3. didOpen with a proc definition + call site — hintable content that
    //    *would* yield parameter-name hints if the feature were enabled.
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///t.tcl","languageId":"tcl","version":1,"#,
        r#""text":"proc add {a b} { expr {$a + $b} }\nadd 1 2\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // 4. textDocument/inlayHint over the whole document.
    let inlay_req = concat!(
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/inlayHint","params":{"#,
        r#""textDocument":{"uri":"file:///t.tcl"},"#,
        r#""range":{"start":{"line":0,"character":0},"end":{"line":2,"character":0}}}}"#,
    );
    client_write
        .write_all(frame(inlay_req).as_bytes())
        .await
        .unwrap();

    // Drain frames until we see id=2 (toss intervening log messages).
    let mut inlay_resp = String::new();
    for _ in 0..5 {
        let frame_body = read_frame(&mut reader).await;
        if frame_body.contains("\"id\":2") {
            inlay_resp = frame_body;
            break;
        }
    }
    assert!(
        !inlay_resp.is_empty(),
        "did not receive id=2 inlay response within 5 frames",
    );
    // Default-off contract: an empty list, not `null` and not populated hints.
    assert!(
        inlay_resp.contains("\"result\":[]"),
        "expected an empty inlay-hint list while the feature is off, got {inlay_resp}",
    );

    // 5. shutdown / exit so the server task can wind down cleanly.
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

    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}
