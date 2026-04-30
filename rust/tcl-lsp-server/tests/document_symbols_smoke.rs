//! End-to-end LSP smoke test for the document-symbol provider.
//!
//! Drives the server through `tower_lsp::LspService` over an
//! in-memory duplex pipe so the test exercises the same
//! `initialize` → `textDocument/didOpen` →
//! `textDocument/documentSymbol` sequence a real client would.
//! Verifies that a `proc` definition shows up in the response as a
//! `Function`-kind symbol.

use std::time::Duration;

use tcl_lsp_server::Backend;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tower_lsp::{LspService, Server};

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
    for _ in 0..5 {
        let frame_body = read_frame(&mut reader).await;
        if frame_body.contains("\"id\":2") {
            sym_resp = frame_body;
            break;
        }
    }
    assert!(
        !sym_resp.is_empty(),
        "did not receive id=2 documentSymbol response within 5 frames",
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

    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}
