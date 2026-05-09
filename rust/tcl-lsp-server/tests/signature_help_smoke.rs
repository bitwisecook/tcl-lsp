//! End-to-end LSP smoke test for `tcl-lsp-server::signature_help`.

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
async fn signature_help_smoke() {
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
    let resp = read_frame(&mut reader).await;
    assert!(resp.contains("\"id\":1"), "initialize response: {resp}");
    assert!(
        resp.contains("\"signatureHelpProvider\""),
        "expected signatureHelpProvider capability: {resp}",
    );

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(500), read_frame(&mut reader)).await;

    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///t.tcl","languageId":"tcl","version":1,"#,
        r#""text":"proc greet {name body} {}\ngreet \n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // Cursor at line 1 char 6 — right after the trailing space following `greet`.
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":"file:///t.tcl"},"position":{"line":1,"character":6}}}"#;
    client_write.write_all(frame(req).as_bytes()).await.unwrap();

    let mut response = String::new();
    for _ in 0..5 {
        let frame_body = read_frame(&mut reader).await;
        if frame_body.contains("\"id\":2") {
            response = frame_body;
            break;
        }
    }
    assert!(
        !response.is_empty(),
        "did not receive id=2 signatureHelp response within 5 frames",
    );
    assert!(
        response.contains("greet"),
        "expected `greet` in signature, got {response}",
    );
    assert!(
        response.contains("\"name\""),
        "expected `name` parameter label, got {response}",
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

    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}
