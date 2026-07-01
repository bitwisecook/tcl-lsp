//! End-to-end LSP smoke tests for diagnostic *delivery* and the
//! reference-count code lens — the two preview regressions #721 and #724.
//!
//! * #721 — a pull-capable client (one advertising `textDocument/diagnostic`)
//!   must receive diagnostics over the pull channel only.  When the server
//!   *also* pushes `textDocument/publishDiagnostics`, clients such as
//!   `vscode-languageclient` route push and pull into two collections and
//!   render every diagnostic twice.  So: no content-bearing push to a
//!   pull-capable client, and the pull report carries exactly one `E003`.
//!
//! * #724 — `codeLens/resolve` must return a clickable
//!   `tcl-lsp.showReferences` command, not an inert title.

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

/// Drain frames until `id` appears in one (returning it) or `max` frames pass.
async fn read_until_id<R>(reader: &mut BufReader<R>, id: &str, max: usize) -> Option<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut all = Vec::new();
    for _ in 0..max {
        match tokio::time::timeout(Duration::from_secs(2), read_frame(reader)).await {
            Ok(body) => {
                let hit = body.contains(id);
                all.push(body);
                if hit {
                    return all.into_iter().next_back();
                }
            }
            Err(_) => break,
        }
    }
    None
}

/// Collect every frame the server emits within `window`.
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

#[tokio::test]
async fn pull_capable_client_is_not_also_pushed() {
    let (client_side, server_side) = tokio::io::duplex(16384);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, mut client_write) = tokio::io::split(client_side);

    let (service, socket) = LspService::new(Backend::new);
    let server = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    let mut reader = BufReader::new(client_read);

    // Advertise pull-diagnostic support (`textDocument.diagnostic`).
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"diagnostic":{"dynamicRegistration":false}}}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let resp = read_until_id(&mut reader, "\"id\":1", 5)
        .await
        .expect("initialize response");
    assert!(
        resp.contains("\"diagnosticProvider\""),
        "server should advertise pull diagnostics: {resp}",
    );

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();

    // `set var 10 10` → one E003 (too many args).  Historically this rendered
    // twice (#721) when push and pull both delivered it.
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///dup.tcl","languageId":"tcl","version":1,"#,
        r#""text":"set var 10 10\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // Within a generous window, the server must not push a *content-bearing*
    // `publishDiagnostics` to this pull-capable client.
    let frames = collect_frames(&mut reader, Duration::from_millis(900)).await;
    for f in &frames {
        if f.contains("textDocument/publishDiagnostics") {
            assert!(
                !f.contains("E003"),
                "pull-capable client must not be pushed diagnostics too (#721): {f}",
            );
        }
    }

    // The pull channel returns exactly one E003.
    let pull = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/diagnostic","params":{"textDocument":{"uri":"file:///dup.tcl"}}}"#;
    client_write
        .write_all(frame(pull).as_bytes())
        .await
        .unwrap();
    let report = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("diagnostic pull response");
    let e003_count = report.matches("E003").count();
    assert_eq!(
        e003_count, 1,
        "exactly one E003 in the pull report, got {e003_count}: {report}",
    );

    let shutdown = r#"{"jsonrpc":"2.0","id":9,"method":"shutdown","params":null}"#;
    client_write
        .write_all(frame(shutdown).as_bytes())
        .await
        .unwrap();
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn push_only_client_still_receives_one_push() {
    let (client_side, server_side) = tokio::io::duplex(16384);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, mut client_write) = tokio::io::split(client_side);

    let (service, socket) = LspService::new(Backend::new);
    let server = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    let mut reader = BufReader::new(client_read);

    // No `textDocument.diagnostic` capability → push-only client.
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

    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///push.tcl","languageId":"tcl","version":1,"#,
        r#""text":"set var 10 10\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // The push-only client gets the diagnostics over `publishDiagnostics`, with
    // exactly one E003 (no duplicate).
    let frames = collect_frames(&mut reader, Duration::from_millis(1200)).await;
    let publishes: Vec<&String> = frames
        .iter()
        .filter(|f| f.contains("textDocument/publishDiagnostics") && f.contains("file:///push.tcl"))
        .collect();
    let with_e003: Vec<&&String> = publishes.iter().filter(|f| f.contains("E003")).collect();
    assert_eq!(
        with_e003.len(),
        1,
        "push-only client should get exactly one E003-bearing publish: {frames:?}",
    );
    assert_eq!(
        with_e003[0].matches("E003").count(),
        1,
        "the publish itself must not duplicate the diagnostic: {}",
        with_e003[0],
    );

    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

#[tokio::test]
async fn code_lens_resolve_returns_show_references_command() {
    let (client_side, server_side) = tokio::io::duplex(16384);
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
    let resp = read_until_id(&mut reader, "\"id\":1", 5)
        .await
        .expect("initialize response");
    assert!(
        resp.contains("\"codeLensProvider\""),
        "server should advertise code lenses: {resp}",
    );

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();

    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///lens.tcl","languageId":"tcl","version":1,"#,
        r#""text":"proc helper {} {}\nhelper\nhelper\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // Request the lenses, then resolve the first one.
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/codeLens","params":{"textDocument":{"uri":"file:///lens.tcl"}}}"#;
    client_write.write_all(frame(req).as_bytes()).await.unwrap();
    let lenses = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("codeLens response");
    assert!(
        lenses.contains("\"qname\":\"::helper\""),
        "expected a lens for ::helper: {lenses}",
    );

    // Resolve a lens with the `{qname, uri}` data the server emitted.
    let resolve = concat!(
        r#"{"jsonrpc":"2.0","id":3,"method":"codeLens/resolve","params":{"#,
        r#""range":{"start":{"line":0,"character":5},"end":{"line":0,"character":11}},"#,
        r#""data":{"qname":"::helper","uri":"file:///lens.tcl"}}}"#,
    );
    client_write
        .write_all(frame(resolve).as_bytes())
        .await
        .unwrap();
    let resolved = read_until_id(&mut reader, "\"id\":3", 8)
        .await
        .expect("codeLens/resolve response");
    assert!(
        resolved.contains("tcl-lsp.showReferences"),
        "resolved lens must carry the show-references command (#724): {resolved}",
    );
    assert!(
        resolved.contains("references"),
        "resolved lens title should show the count: {resolved}",
    );

    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}

/// The analyser's `catch`-body walk must surface end-to-end: an unbraced
/// `expr` inside a `catch { … }` produces a `W100` (expression not braced) in
/// the pull-diagnostic report, the same as it would at the top level. Guards
/// the parity fix that made `handle_catch_command` recurse into `args[0]`.
#[tokio::test]
async fn catch_body_diagnostics_are_delivered() {
    let (client_side, server_side) = tokio::io::duplex(16384);
    let (server_read, server_write) = tokio::io::split(server_side);
    let (client_read, mut client_write) = tokio::io::split(client_side);

    let (service, socket) = LspService::new(Backend::new);
    let server = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    let mut reader = BufReader::new(client_read);

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"diagnostic":{"dynamicRegistration":false}}}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let _ = read_until_id(&mut reader, "\"id\":1", 5)
        .await
        .expect("initialize response");
    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();

    // `catch { expr $x+1 }` — the unbraced `expr` inside the catch body is W100.
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///catch.tcl","languageId":"tcl","version":1,"#,
        r#""text":"catch { expr $x+1 }\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    let pull = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/diagnostic","params":{"textDocument":{"uri":"file:///catch.tcl"}}}"#;
    client_write
        .write_all(frame(pull).as_bytes())
        .await
        .unwrap();
    let report = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("diagnostic pull response");
    assert!(
        report.contains("W100"),
        "expected W100 from the catch body in the pull report, got {report}",
    );

    let shutdown = r#"{"jsonrpc":"2.0","id":9,"method":"shutdown","params":null}"#;
    client_write
        .write_all(frame(shutdown).as_bytes())
        .await
        .unwrap();
    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}
