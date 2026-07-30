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

//! End-to-end LSP smoke tests for diagnostic *delivery* and the
//! reference-count code lens — the two preview regressions #721 and #724.
//!
//! * #721 — pull diagnostics are opt-in.  When a server advertises
//!   `diagnosticProvider`, clients such as `vscode-languageclient` switch to
//!   pull mode and stop honouring push (or, if they route both, render every
//!   diagnostic twice).  So by default the server does NOT advertise
//!   `diagnosticProvider`, and push is the sole channel — a pull-capable
//!   client still receives exactly one content-bearing `publishDiagnostics`.
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

/// Read frames into `sink` until the accumulated frames satisfy `reached`, and
/// report whether they did before `backstop` expired.
///
/// The counterpart of [`collect_frames`] for anything that has a *terminal
/// signal* rather than a quiet period: instead of guessing how long the server
/// needs, wait for the thing that says it is done. A fixed collection window is
/// a wall-clock bet on the server's speed, so on a machine short of CPU it ends
/// the collection early and turns a correct server into a missing publish; the
/// backstop here is only a hang guard, never the synchronisation. Every frame
/// read is kept, so the caller still sees the true arrival order.
async fn drain_until<R>(
    reader: &mut BufReader<R>,
    sink: &mut Vec<String>,
    backstop: Duration,
    reached: impl Fn(&[String]) -> bool,
) -> bool
where
    R: tokio::io::AsyncRead + Unpin,
{
    let deadline = tokio::time::Instant::now() + backstop;
    loop {
        if reached(sink) {
            return true;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match tokio::time::timeout(remaining, read_frame(reader)).await {
            Ok(body) => sink.push(body),
            Err(_) => return reached(sink),
        }
    }
}

#[tokio::test]
async fn pull_capable_client_still_gets_push_by_default() {
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

    // Advertise pull-diagnostic support (`textDocument.diagnostic`).  Pull
    // diagnostics are nonetheless opt-in: the server must NOT advertise
    // `diagnosticProvider`, so a pull-capable client that isn't told to pull
    // keeps receiving push.  (Advertising it would flip most clients to
    // pull-only and silently disable the richer push pipeline — the inverse of
    // the #721 double-render hazard.)
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"diagnostic":{"dynamicRegistration":false}}}}}"#;
    client_write
        .write_all(frame(init).as_bytes())
        .await
        .unwrap();
    let resp = read_until_id(&mut reader, "\"id\":1", 5)
        .await
        .expect("initialize response");
    assert!(
        !resp.contains("\"diagnosticProvider\""),
        "pull diagnostics are opt-in and must not be advertised by default: {resp}",
    );

    let initialized = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    client_write
        .write_all(frame(initialized).as_bytes())
        .await
        .unwrap();

    // `set var 10 10` → one E003 (too many args).
    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///dup.tcl","languageId":"tcl","version":1,"#,
        r#""text":"set var 10 10\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    // Push is the sole channel: the server must deliver exactly one
    // content-bearing `publishDiagnostics` carrying the E003, even to this
    // pull-capable client (which, absent a `diagnosticProvider`, never pulls).
    let frames = collect_frames(&mut reader, Duration::from_millis(1500)).await;
    let e003_pushes = frames
        .iter()
        .filter(|f| f.contains("textDocument/publishDiagnostics") && f.contains("E003"))
        .count();
    assert_eq!(
        e003_pushes, 1,
        "expected exactly one pushed E003 publish, got {e003_pushes}: {frames:?}",
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
    server.abort();
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
    server.abort();
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
    server.abort();
}

/// The analyser's `catch`-body walk must surface end-to-end: an unbraced
/// `expr` inside a `catch { … }` produces a `W100` (expression not braced) in
/// the pull-diagnostic report, the same as it would at the top level. Guards
/// the fix that made `handle_catch_command` recurse into `args[0]`.
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
    server.abort();
}

/// The document `rapid_edits_deliver_diagnostics_in_version_order_without_loss`
/// drives.
const ORDER_URI: &str = "file:///order.tcl";

/// Hang guard for that test's drains. Only a backstop: every wait is on a real
/// signal, so reaching it means the server stopped publishing altogether.
const ORDER_BACKSTOP: Duration = Duration::from_mins(1);

/// Send a full-text replace of [`ORDER_URI`] at `version`.
async fn send_full_replace<W>(writer: &mut W, text: &str, version: i64)
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let change = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{ORDER_URI}","version":{version}}},"contentChanges":[{{"text":{text:?}}}]}}}}"#,
    );
    writer.write_all(frame(&change).as_bytes()).await.unwrap();
}

/// Whether `frames` already carries a version-tagged publish for [`ORDER_URI`]
/// at or beyond `version`.
///
/// `>=` rather than `==`: the per-URI worker reads the document's *current*
/// state at drain time, so a publish tagged with a later version is equally
/// proof that this one has been through the worker.
fn published_at_least(frames: &[String], version: i64) -> bool {
    version_tagged_publishes(frames, ORDER_URI)
        .iter()
        .any(|(v, _)| *v >= version)
}

/// Every version-tagged `publishDiagnostics` for `uri` in `frames`, in arrival
/// order, paired with the raw frame.
fn version_tagged_publishes(frames: &[String], uri: &str) -> Vec<(i64, String)> {
    frames
        .iter()
        .filter(|f| f.contains("textDocument/publishDiagnostics") && f.contains(uri))
        .filter_map(|f| {
            let v: serde_json::Value = serde_json::from_str(f).ok()?;
            let version = v.get("params")?.get("version")?.as_i64()?;
            Some((version, f.clone()))
        })
        .collect()
}

/// Delivery-invariant regression guard (see the `main.rs` delivery invariant).
///
/// Diagnostics delivery relies on the transport being bounded and
/// backpressured: `publish_diagnostics(..).await` suspends until the client
/// drains, so a client that falls behind never makes the server reorder
/// publishes or drop the final state. Two things are asserted:
///   1. **Ordering**: published `version`s are monotonically non-decreasing. A
///      regression that makes delivery fire-and-forget (a detached `tokio::spawn`
///      per publish, dropping backpressure) would let publishes race and land
///      out of version order — this fails then.
///   2. **No loss of final state**: the last edit's version is published with
///      its diagnostic. A drop-on-full transport would lose it — this fails then.
///
/// # Why this is in two phases (issue #1082)
///
/// The ordering guard needs at least two version-tagged publishes to compare,
/// and it used to get them by spacing the edits past `DIAGNOSTICS_DEBOUNCE`
/// with a fixed sleep and then collecting frames for a fixed 2.5 s window. Both
/// halves were wall-clock bets on the server's speed, and both lose on a
/// machine short of CPU: the collection window closes while publishes are still
/// being produced, the burst arrives at the per-URI worker inside one debounce
/// window and coalesces, and the test trips *its own* "≥2 in-flight publishes"
/// precondition. That is a starved scheduler failing a delivery test, not a
/// delivery regression.
///
/// So the two properties are now driven separately, each against a signal
/// rather than a clock:
///
/// * **Phase A — ordering.** Each edit is followed by draining frames until a
///   publish for that version (or later) actually arrives. The barrier is the
///   publish itself, so the next edit cannot reach the worker inside the same
///   debounce window and the burst *cannot* coalesce: the number of ordered,
///   version-tagged publishes is a structural property of the phase, not a race
///   the machine can lose. Frames are kept as they are read, so the ordering
///   check still sees true arrival order.
/// * **Phase B — concurrent publishes, and no loss under backpressure.** A
///   burst fired with the client deliberately **not reading**, so edits pile up
///   on a worker that is already running and the publishes queue in the
///   transport rather than being consumed one at a time. This is the phase that
///   observes publishes that were *produced concurrently*: Phase A's barrier
///   deliberately forbids that, so the ordering assertion would otherwise only
///   ever see a serialized stream (review of #1089). Here coalescing is expected
///   and fine — under load the burst may collapse to a single publish, and the
///   number of publishes is therefore never asserted, only that whatever arrives
///   arrives in non-decreasing version order and that the *final* state is not
///   dropped. Both are drained for by a terminal signal (the final version's
///   publish), not a fixed window, so neither claim depends on the machine.
///
/// # What the ordering assertion does and does not prove (measured)
///
/// The pre-#1082 version of this test claimed that a fire-and-forget delivery
/// regression "would reorder them". That claim was never true, and the claim —
/// not the coverage — is what changed here. Measured against an actual
/// regression patch (`cache_and_deliver` detaching the send into a
/// `tokio::spawn`, releasing the `documents` lock before it):
///
/// * the pre-#1082 test passed 10/10, and this one passes 10/10;
/// * six further burst shapes — current-thread *and* 4-worker multi-thread
///   runtimes, transport buffers from 256 B to 64 KiB (so the send genuinely
///   parks on backpressure), 6–12 unread edits, with and without a `did_close`
///   chaser — never produced an out-of-order publish (0/20 non-monotone on the
///   most aggressive shape).
///
/// That is structural, not luck: `tower-lsp-server`'s `Client` is a
/// `futures::mpsc` channel drained by one forwarder task, each publish is a
/// single `send`, and mpsc preserves send-initiation order across senders — so
/// detaching the send cannot reorder version-tagged publishes. What the
/// regression *does* change is which publishes survive the currency guard (it
/// drops more intermediates), and that is indistinguishable from legitimate
/// coalescing, so it cannot be asserted without reintroducing a load-dependent
/// flake.
///
/// The ordering assertion is therefore kept as what it actually is — a cheap
/// invariant over the real delivered stream, exercised over concurrently
/// produced publishes in Phase B — and not advertised as the guard for the
/// fire-and-forget regression. The guard for *that* is the invariant comment in
/// `main.rs` and review of the delivery path.
#[tokio::test]
async fn rapid_edits_deliver_diagnostics_in_version_order_without_loss() {
    let (client_side, server_side) = tokio::io::duplex(65536);
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
    client_write
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#).as_bytes())
        .await
        .unwrap();

    let did_open = concat!(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"#,
        r#""uri":"file:///order.tcl","languageId":"tcl","version":1,"#,
        r#""text":"set var 10 10\n"}}}"#,
    );
    client_write
        .write_all(frame(did_open).as_bytes())
        .await
        .unwrap();

    let mut frames: Vec<String> = Vec::new();

    // Phase A — ordering. Full-replace edits alternating clean / E003 so
    // consecutive versions differ (no salsa early-cutoff skip), each followed by
    // a drain up to that version's publish. Barriering on the publish (not on a
    // sleep) is what makes the separate-publish-per-version property structural
    // rather than a timing race: the next edit cannot reach the per-URI worker
    // inside the debounce window that is still serving this one.
    let ordering_edits = [
        ("set var 10\n", 2),
        ("set var 10 10\n", 3),
        ("set var 10\n", 4),
        ("set var 10 10\n", 5),
        ("set var 10\n", 6),
        ("set a 1 2 3 4\n", 7),
    ];
    for (text, version) in ordering_edits {
        send_full_replace(&mut client_write, text, version).await;
        let arrived = drain_until(&mut reader, &mut frames, ORDER_BACKSTOP, |f| {
            published_at_least(f, version)
        })
        .await;
        assert!(
            arrived,
            "no publishDiagnostics at version >= {version} within {ORDER_BACKSTOP:?} — the \
             per-URI diagnostics worker stopped publishing: {frames:?}",
        );
    }

    // Phase B — concurrent publishes + no loss under backpressure. A burst with
    // the client deliberately NOT reading, so the edits pile onto a worker that
    // is already running and the publishes queue in the transport instead of
    // being consumed one at a time. Long enough (and alternating clean / E003,
    // so consecutive versions differ) that the worker gets the chance to produce
    // several publishes while nothing is draining them — that, not a count, is
    // the point: it is the only phase where the ordering assertion below sees
    // publishes that were produced concurrently. How many survive coalescing is
    // the machine's business and is never asserted.
    let first_burst_version = 8;
    let burst = [
        ("set var 10\n", 8),
        ("set var 10 10\n", 9),
        ("set var 10\n", 10),
        ("set var 10 10\n", 11),
        ("set var 10\n", 12),
        ("set a 1 2 3 4 5\n", 13),
    ];
    for (text, version) in burst {
        send_full_replace(&mut client_write, text, version).await;
    }
    let final_version = burst[burst.len() - 1].1;
    let arrived = drain_until(&mut reader, &mut frames, ORDER_BACKSTOP, |f| {
        version_tagged_publishes(f, ORDER_URI)
            .iter()
            .any(|(v, _)| *v == final_version)
    })
    .await;

    assert_delivery_invariants(&frames, first_burst_version, final_version, arrived);

    let exit = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
    client_write
        .write_all(frame(exit).as_bytes())
        .await
        .unwrap();
    drop(client_write);
    server.abort();
}

/// The two delivery invariants, checked over the frames both phases collected.
///
/// `arrived` is whether the Phase B drain reached `final_version`'s publish;
/// `first_burst_version` separates Phase A's serialized stream from Phase B's
/// concurrently produced one.
fn assert_delivery_invariants(
    frames: &[String],
    first_burst_version: i64,
    final_version: i64,
    arrived: bool,
) {
    let publishes = version_tagged_publishes(frames, ORDER_URI);
    let versions: Vec<i64> = publishes.iter().map(|(v, _)| *v).collect();

    // Tripwire: a single publish makes the ordering `windows(2)` loop vacuous
    // (zero pairs). Phase A guarantees one publish per version, so this can only
    // trip if the publish-barrier contract itself broke — a real failure, not a
    // load artefact.
    assert!(
        publishes.len() >= 2,
        "expected ≥2 version-tagged publishes to exercise delivery ordering, \
         got {versions:?}: {frames:?}",
    );
    // 1) Ordering: never a lower version after a higher one — over the whole
    //    delivered stream, and (called out separately because it is the half
    //    that observes *concurrently produced* publishes) over Phase B's.
    for pair in publishes.windows(2) {
        assert!(
            pair[0].0 <= pair[1].0,
            "publishes must arrive in non-decreasing version order: {versions:?}",
        );
    }
    let burst_versions: Vec<i64> = versions
        .iter()
        .copied()
        .filter(|v| *v >= first_burst_version)
        .collect();
    for pair in burst_versions.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "the unread burst's publishes must arrive in non-decreasing version order — \
             versions may skip (coalescing is expected here) but never go backwards: \
             {burst_versions:?} (whole stream: {versions:?})",
        );
    }
    // 2) No loss of the final state: the last edit's version must arrive with
    //    its diagnostic.
    assert!(
        arrived,
        "the final edit's diagnostics (version {final_version}) must be delivered — never \
         dropped: {versions:?}",
    );
    let final_publish = publishes
        .iter()
        .rev()
        .find(|(v, _)| *v == final_version)
        .expect("the drain barrier proved this publish arrived");
    assert!(
        final_publish.1.contains("E003"),
        "the final publish must carry the final state's E003: {}",
        final_publish.1,
    );
}
