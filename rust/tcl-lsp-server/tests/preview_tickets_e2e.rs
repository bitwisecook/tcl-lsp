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
//! * #867 — `lassign $point x y z` writes list *elements*, so the targets must
//!   not inherit `lassign`'s `List` return type and fire S100 in `[expr]`.
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
async fn lassign_targets_not_flagged_s100_shimmer_e2e() {
    // Issue #867, verbatim reproducer: `lassign` destructures a list into
    // per-element locals whose intrep is not the `List` the command returns.
    // Pre-fix the targets were typed LIST, so the arithmetic on the last line
    // fired S100 "variable has list intrep used in arithmetic expression".
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///lassign.tcl",
        "set point [list 1 2 3]\n\
         lassign $point x y z\n\
         set offset [expr {$x + $y + $z}]\n",
    )
    .await;
    let diags = published_codes(&mut reader, "file:///lassign.tcl").await;
    assert!(
        !diags.contains("S100"),
        "lassign targets are list elements, not lists — no S100 shimmer (#867): {diags}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn regexp_capture_not_flagged_s100_shimmer_e2e() {
    // Sibling of #867: a `regexp` capture holds a matched substring, not the
    // Int match count the command returns.  Comparing it as a string must not
    // fire S100 "numeric variable used in string comparison".
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///regexp.tcl",
        "set s \"abc\"\n\
         regexp {(.)} $s letter\n\
         if {$letter eq \"a\"} { puts yes }\n",
    )
    .await;
    let diags = published_codes(&mut reader, "file:///regexp.tcl").await;
    assert!(
        !diags.contains("S100"),
        "regexp capture is a string, not the Int count — no S100 (#867): {diags}",
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

/// Regression for a multi-parameter span bug found by differential audit
/// against real-world code (a `reduce`/`foldl`-style proc): the token-span
/// slice feeding `param_name_spans` was missing the closing `}` a braced
/// param-list token's span never included, so its own outer-brace check
/// silently misfired and swallowed every parameter after the first — only
/// the *first* formal ever got a real declaration span; every later one
/// fell back to whatever span the caller defaulted to.
///
/// The sibling `method_param_definition_resolves_to_name_e2e` test above
/// already used a two-parameter method (`arg1 arg2`) but only ever checked
/// `arg1` — the one parameter this bug never broke — so it passed both
/// before and after the regression was introduced. These tests exist
/// specifically to close that blind spot: every parameter must be checked
/// individually, not just the first.
#[tokio::test]
async fn proc_param_definition_resolves_to_name_for_every_param_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    // `proc reduce {f z xs} { ... }` — 3 formals; check the 2nd and 3rd, each
    // used in the body so go-to-definition has a call site to start from.
    did_open(
        &mut writer,
        "file:///reduce.tcl",
        "proc reduce {f z xs} {\n    puts $z\n    return $xs\n}\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    // `$z` usage is on line 1 (`    puts $z`); the 'z' byte (after `$`) is
    // at character 10. 'z' is declared on line 0 at character 15 (`proc
    // reduce {f z xs}`, 0-based: p-r-o-c- -r-e-d-u-c-e- -{-f- -z...). The
    // pre-fix bug resolved this to the *proc name*'s span instead (line 0,
    // character 5, "reduce").
    let req_z = r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///reduce.tcl"},"position":{"line":1,"character":10}}}"#;
    writer.write_all(frame(req_z).as_bytes()).await.unwrap();
    let resp_z = read_until_id(&mut reader, "\"id\":3", 8)
        .await
        .expect("definition response for z");
    assert!(
        resp_z.contains(r#""line":0"#) && resp_z.contains(r#""character":15"#),
        "2nd param 'z' must resolve to its own name (line 0, char 15), \
         not fall back to the proc name: {resp_z}",
    );
    assert!(
        !resp_z.contains(r#""character":5"#),
        "2nd param 'z' must not resolve to the proc name 'reduce' \
         (char 5) — the pre-fix bug's exact wrong answer: {resp_z}",
    );

    // `$xs` usage is on line 2 (`    return $xs`); the 'x' byte (after `$`)
    // is at character 12. 'xs' is declared on line 0 at character 17.
    let req_xs = r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///reduce.tcl"},"position":{"line":2,"character":12}}}"#;
    writer.write_all(frame(req_xs).as_bytes()).await.unwrap();
    let resp_xs = read_until_id(&mut reader, "\"id\":4", 8)
        .await
        .expect("definition response for xs");
    assert!(
        resp_xs.contains(r#""line":0"#) && resp_xs.contains(r#""character":17"#),
        "3rd param 'xs' must resolve to its own name (line 0, char 17), \
         not fall back to the proc name or merge with 'z': {resp_xs}",
    );
    assert!(
        !resp_xs.contains(r#""character":15"#) && !resp_xs.contains(r#""character":5"#),
        "3rd param 'xs' must not share 'z's span or fall back to the proc \
         name — before the fix every param past the first collapsed onto \
         the same wrong fallback span: {resp_xs}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn method_param_definition_resolves_to_name_for_second_param_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    // Same source shape as `method_param_definition_resolves_to_name_e2e`,
    // but this time checking `arg2` — the parameter that bug actually broke.
    did_open(
        &mut writer,
        "file:///oo2.tcl",
        "oo::class create C {\n    method m {arg1 arg2} {\n        puts $arg2\n    }\n}\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;
    // `$arg2` usage is on line 2, character 14 (after `$`).
    let req = r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///oo2.tcl"},"position":{"line":2,"character":14}}}"#;
    writer.write_all(frame(req).as_bytes()).await.unwrap();
    let resp = read_until_id(&mut reader, "\"id\":5", 8)
        .await
        .expect("definition response");
    // `arg2` is declared on line 1 (`    method m {arg1 arg2} {`) — the
    // pre-fix bug fell back to `m`'s method-name span instead.
    assert!(
        resp.contains(r#""line":1"#),
        "2nd method param 'arg2' must resolve to its own declaration on \
         line 1, not fall back to the method name or body: {resp}",
    );
    assert!(
        !resp.contains(r#""line":3"#),
        "2nd method param 'arg2' must not resolve to the whole method body: {resp}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn nested_shadow_of_builtin_does_not_leak_across_files_e2e() {
    // Regression for a bug found by differential audit against tcllib's
    // rest.tcl (`::rest::describe`): a `proc ::set {...}` written *inside*
    // another proc's body — to temporarily intercept `set` calls, then
    // `rename` the builtin back — permanently outranked the real builtin for
    // hover / go-to-definition everywhere in the *workspace*, including an
    // entirely unrelated file that never calls the proc containing the
    // shadow at all.
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///shadow_lib.tcl",
        "proc ::demo::describe {} {\n    rename ::set ::_set\n    proc ::set {v val} { ::_set $v $val }\n    rename ::set {}\n    rename ::_set ::set\n}\n",
    )
    .await;
    did_open(
        &mut writer,
        "file:///shadow_main.tcl",
        "set final_check restored\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    // `set` in shadow_main.tcl never runs anywhere near `describe` — it is
    // unambiguously the builtin.
    let def_req = r#"{"jsonrpc":"2.0","id":7,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///shadow_main.tcl"},"position":{"line":0,"character":0}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":7", 8)
        .await
        .expect("definition response");
    assert!(
        !def_resp.contains("shadow_lib.tcl"),
        "an unrelated file's nested shadow of `set` must never leak into \
         another file's go-to-definition: {def_resp}",
    );

    let hover_req = r#"{"jsonrpc":"2.0","id":8,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///shadow_main.tcl"},"position":{"line":0,"character":0}}}"#;
    writer.write_all(frame(hover_req).as_bytes()).await.unwrap();
    let hover_resp = read_until_id(&mut reader, "\"id\":8", 8)
        .await
        .expect("hover response");
    assert!(
        hover_resp.contains("built-in") || hover_resp.contains("Built-in"),
        "hover must describe the real `set` builtin, not the unrelated \
         file's shadow proc: {hover_resp}",
    );
    assert!(
        !hover_resp.contains("::_set"),
        "hover must not surface the shadow proc's body: {hover_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// LSP position `(line, character)` compared lexicographically, matching
/// how a `Range` orders `start`/`end`.
fn pos_le(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    let al = a["line"].as_u64().unwrap_or(0);
    let bl = b["line"].as_u64().unwrap_or(0);
    let ac = a["character"].as_u64().unwrap_or(0);
    let bc = b["character"].as_u64().unwrap_or(0);
    (al, ac) <= (bl, bc)
}

/// Recursively assert every `DocumentSymbol`'s `range` contains each of its
/// own `children`'s `range` — the exact structural invariant a merged
/// `ClassDef` from two unrelated `oo::define $class …` call sites violates
/// (a child method's range lying textually outside its claimed parent's).
fn assert_symbol_ranges_well_nested(symbols: &[serde_json::Value], path: &str) {
    for sym in symbols {
        let name = sym["name"].as_str().unwrap_or("<unnamed>");
        let here = format!("{path}/{name}");
        if let Some(range) = sym.get("range") {
            for child in sym["children"].as_array().into_iter().flatten() {
                let Some(crange) = child.get("range") else {
                    continue;
                };
                assert!(
                    pos_le(&range["start"], &crange["start"])
                        && pos_le(&crange["end"], &range["end"]),
                    "{here}: child {:?} range {crange} escapes parent range {range}",
                    child["name"],
                );
            }
        }
        assert_symbol_ranges_well_nested(
            sym["children"].as_array().map_or(&[][..], Vec::as_slice),
            &here,
        );
    }
}

#[tokio::test]
async fn dynamic_oo_define_targets_never_corrupt_the_document_symbol_tree_e2e() {
    // Regression for a bug found by differential audit against clay.tcl's
    // `current_class`-based Ensemble DSL: two lexically unrelated procs each
    // write `oo::define $class method ...` with a same-named local `class`,
    // each adding their own method. Before the fix, both calls computed the
    // identical raw-text key `$class`, so the second `oo::define` merged its
    // method into the first's `ClassDef` (and vice versa via the
    // dual-registration into the scope tree) — producing a document symbol
    // whose child method range sat outside its own parent's, a
    // self-contradictory structure checkable from the response alone.
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///dynclass.tcl",
        "proc addFoo {} {\n    set class ::A\n    oo::define $class method foo {} { return foo }\n}\n\
         proc addBar {} {\n    set class ::B\n    oo::define $class method bar {} { return bar }\n}\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    let sym_req = r#"{"jsonrpc":"2.0","id":12,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///dynclass.tcl"}}}"#;
    writer.write_all(frame(sym_req).as_bytes()).await.unwrap();
    let sym_resp = read_until_id(&mut reader, "\"id\":12", 8)
        .await
        .expect("documentSymbol response");
    let parsed: serde_json::Value =
        serde_json::from_str(&sym_resp).expect("valid JSON documentSymbol response");
    let symbols = parsed["result"].as_array().cloned().unwrap_or_default();
    assert_symbol_ranges_well_nested(&symbols, "");

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn dynamic_namespace_eval_targets_never_cross_resolve_e2e() {
    // Regression for a bug found by differential audit against irc.tcl's
    // per-connection `namespace eval $name { … }` idiom: two lexically
    // unrelated procs each open a dynamic namespace with a same-named local
    // `name`, each defining their own `helper`. Before the fix both blocks'
    // scope shared the literal text `$name`, so a call inside one script
    // could resolve to the *other* proc's unrelated `helper`.
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///dynns.tcl",
        "proc setupA {} {\n    set name connA\n    namespace eval $name {\n        proc helper {} { return A }\n    }\n}\n\
         proc setupB {} {\n    set name connB\n    namespace eval $name {\n        proc helper {} { return B }\n        helper\n    }\n}\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    // The `helper` call on line 10 (`        helper`), inside setupB's own
    // dynamic namespace body.
    let def_req = r#"{"jsonrpc":"2.0","id":13,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///dynns.tcl"},"position":{"line":10,"character":10}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":13", 8)
        .await
        .expect("definition response");
    // setupB's own `helper` is declared on line 9, not setupA's on line 3.
    assert!(
        def_resp.contains(r#""line":9"#),
        "the call inside setupB's dynamic namespace must resolve to its \
         own helper on line 9, not setupA's on line 3: {def_resp}",
    );
    assert!(
        !def_resp.contains(r#""line":3"#),
        "must never cross-resolve into the other dynamic namespace's helper: {def_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn interp_handle_eval_isolates_from_a_sibling_interpreter_e2e() {
    // Regression for a bug found by differential audit against
    // docstrip_util.tcl: `NAME eval { … }` (the interpreter's own object
    // command, `interp create sandbox` binds `sandbox` itself as a callable
    // command) is the far more common real-world spelling of `interp eval
    // NAME { … }`, but was only ever recognised in the literal `interp eval`
    // form — the handle form's body was neither isolated nor walked, so a
    // call inside it fell back to a scope-blind, file-wide "any proc
    // anywhere with this bare name" match. With two separate interpreters
    // each defining their own `helper` — one via literal `interp eval`, one
    // via the handle form — a call inside the handle-form script must
    // resolve to *its own* interpreter's helper, never the other's.
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///twointerp.tcl",
        "interp create sandboxA\ninterp create sandboxB\n\
         interp eval sandboxA { proc helper {} { return A } }\n\
         sandboxB eval {\n    proc helper {} { return B }\n    helper\n}\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    // The `helper` call on line 5 (`    helper`), inside sandboxB's own
    // script.
    let def_req = r#"{"jsonrpc":"2.0","id":11,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///twointerp.tcl"},"position":{"line":5,"character":5}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":11", 8)
        .await
        .expect("definition response");
    // sandboxB's own `helper` is declared on line 4, not sandboxA's on
    // line 2.
    assert!(
        def_resp.contains(r#""line":4"#),
        "the call inside sandboxB's script must resolve to sandboxB's own \
         helper on line 4, not sandboxA's on line 2: {def_resp}",
    );
    assert!(
        !def_resp.contains(r#""line":2"#),
        "must never cross-resolve into the other interpreter's helper: {def_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn qualified_namespace_var_definition_resolves_from_anywhere_in_the_file_e2e() {
    // Regression for a bug found by differential audit against tcllib's
    // defer.tcl (`$::defer::idVar`) / uri.tcl: a fully `::`-qualified
    // variable reference never resolved at all — `lookup_var_in_scope_chain`
    // only special-cased bare names; a qualified name fell through to a
    // literal scope-chain key match that could never hit, since `VarDef`
    // entries are keyed by their bare name inside the namespace's own scope,
    // not by the fully qualified string. Both hover and go-to-definition
    // share that one resolver, so both must pick up the fix.
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///qualvar.tcl",
        "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;

    // Cursor on `v` inside `$::simple::v` (line 3, char 16).
    let def_req = r#"{"jsonrpc":"2.0","id":9,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///qualvar.tcl"},"position":{"line":3,"character":16}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":9", 8)
        .await
        .expect("definition response");
    // `v` is declared on line 1 (`    variable v "hello"`), character 13.
    assert!(
        def_resp.contains(r#""line":1"#) && def_resp.contains(r#""character":13"#),
        "a fully-qualified $::simple::v must resolve to the namespace's \
         `variable v` declaration on line 1: {def_resp}",
    );

    let hover_req = r#"{"jsonrpc":"2.0","id":10,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///qualvar.tcl"},"position":{"line":3,"character":16}}}"#;
    writer.write_all(frame(hover_req).as_bytes()).await.unwrap();
    let hover_resp = read_until_id(&mut reader, "\"id\":10", 8)
        .await
        .expect("hover response");
    assert!(
        hover_resp.contains("Variable"),
        "hover must also resolve the qualified variable, sharing the same \
         resolver as go-to-definition: {hover_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

#[tokio::test]
async fn apply_lambda_param_definition_resolves_to_name_for_second_param_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    // `apply {{a b} {...}} args...` — an inline lambda; the 3rd
    // `param_name_spans` call site the same bug affected. Must be a
    // top-level statement: `apply` is only specially recognised as the head
    // of its own command, not nested inside another command substitution
    // (e.g. `puts [apply ...]` walks `{a b}` as an ordinary nested command
    // instead, per the existing `apply {dir {...}}`-style tests).
    did_open(
        &mut writer,
        "file:///apply.tcl",
        "apply {{a b} {return $b}} 1 2\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;
    // `$b` usage is at line 0, character 22 (after `$`, inside the lambda body).
    let req = r#"{"jsonrpc":"2.0","id":6,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///apply.tcl"},"position":{"line":0,"character":22}}}"#;
    writer.write_all(frame(req).as_bytes()).await.unwrap();
    let resp = read_until_id(&mut reader, "\"id\":6", 8)
        .await
        .expect("definition response");
    // `b` is declared at character 10 (`apply {{a b} ...`, 0-based).
    assert!(
        resp.contains(r#""character":10"#),
        "lambda's 2nd param 'b' must resolve to its own name (char 10): {resp}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TP — issue #923 idx 3: `rename OLD NEW` gave up entirely (treated as
/// unconditionally dynamic, no binding recorded) whenever either argument
/// contained `$`/`[`, even when the value was a compile-time constant.
/// `set old ::foo_impl; rename $old ::foo` is the finding's own simplest
/// repro; before the fix, go-to-definition on the resulting `::foo` call
/// returned an empty result.
#[tokio::test]
async fn dynamic_but_resolvable_rename_definition_follows_the_move_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///rename.tcl",
        "proc ::foo_impl {} { return \"impl body\" }\nset old ::foo_impl\nrename $old ::foo\nputs [::foo]\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;
    // `::foo` in `puts [::foo]` (line 3, char 7) — reached only through a
    // dynamic-but-constant-foldable `rename $old ::foo`.
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///rename.tcl"},"position":{"line":3,"character":7}}}"#;
    writer.write_all(frame(req).as_bytes()).await.unwrap();
    let resp = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("definition response");
    assert!(
        !resp.contains(r#""result":[]"#) && !resp.contains(r#""result":null"#),
        "definition through a resolvable dynamic rename must not be empty: {resp}",
    );
    assert!(
        resp.contains(r#""line":0"#),
        "must resolve to ::foo_impl's declaration on line 0: {resp}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TN — issue #923 idx 3 regression guard: a rename argument that is
/// dynamic and genuinely unresolvable (piped through `gets`, never a
/// tracked compile-time constant) must still abstain — no false
/// go-to-definition claim.
#[tokio::test]
async fn genuinely_dynamic_rename_still_abstains_from_definition_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///rename_dynamic.tcl",
        "proc ::foo_impl {} { return \"impl body\" }\nset old [gets stdin]\nrename $old ::foo\nputs [::foo]\n",
    )
    .await;
    let _ = collect_frames(&mut reader, Duration::from_millis(400)).await;
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///rename_dynamic.tcl"},"position":{"line":3,"character":7}}}"#;
    writer.write_all(frame(req).as_bytes()).await.unwrap();
    let resp = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("definition response");
    assert!(
        resp.contains(r#""result":[]"#) || resp.contains(r#""result":null"#),
        "definition through a genuinely-dynamic rename must abstain, not guess: {resp}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// FP — issue #923 idx 110: `namespace eval $ns [list namespace unknown
/// $handler]` (the tcllib `namespacex::hook::Set` idiom) installs a
/// per-namespace unknown-command handler, but the `[...]` body is a
/// `Cmd`-kind token that the analyser's literal-`{...}`-only body walk
/// never enters — so the installer used to be invisible and a call the
/// handler chain resolves at runtime drew a false W123.
#[tokio::test]
async fn list_wrapped_namespace_unknown_installer_suppresses_w123_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///nsunknown.tcl",
        "proc handler {args} { puts $args }\nnamespace eval ::target [list namespace unknown handler]\nmystery_cmd 1\n",
    )
    .await;
    let diags = published_codes(&mut reader, "file:///nsunknown.tcl").await;
    assert!(
        !diags.contains("W123"),
        "a resolvable list-wrapped namespace-unknown installer must suppress W123: {diags}",
    );
    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// FP/TP — issue #923 idx 113: a bareword call to a sibling `TclOO`
/// method/classmethod/property inside another method's body only
/// actually dispatches when `oo::Helpers::link` exposed it that way;
/// `lookup_class_member`/`class_member_hover_text` used to match
/// unconditionally. `link foo` (constructor) makes `foo` genuinely
/// bareword-callable; without it, the call errors "invalid command
/// name" in real tclsh and go-to-definition/hover must abstain, not
/// guess.
#[tokio::test]
async fn class_member_bareword_call_requires_link_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///link.tcl",
        "oo::class create Widget {\n    method foo {x} { return \"foo-called:$x\" }\n    method bar {} { return [foo 42] }\n}\n",
    )
    .await;
    // `foo` inside `bar`'s body (`method bar {} { return [foo 42] }`) —
    // line 2, char 28 lands on the `f` of `foo`.
    let def_req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///link.tcl"},"position":{"line":2,"character":28}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":2", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp.contains(r#""result":[]"#) || def_resp.contains(r#""result":null"#),
        "un-linked bareword sibling call must abstain from definition: {def_resp}",
    );
    let hover_req = r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///link.tcl"},"position":{"line":2,"character":28}}}"#;
    writer.write_all(frame(hover_req).as_bytes()).await.unwrap();
    let hover_resp = read_until_id(&mut reader, "\"id\":3", 8)
        .await
        .expect("hover response");
    assert!(
        hover_resp.contains(r#""result":null"#),
        "un-linked bareword sibling call must abstain from hover: {hover_resp}",
    );

    did_open(
        &mut writer,
        "file:///link2.tcl",
        "oo::class create Widget {\n    constructor {} { link foo }\n    method foo {x} { return \"foo-called:$x\" }\n    method bar {} { return [foo 42] }\n}\n",
    )
    .await;
    // `foo` inside `bar`'s body — now line 3, char 28 (constructor line
    // shifted everything down by one; same column as above, the body
    // text itself is unchanged).
    let def_req2 = r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///link2.tcl"},"position":{"line":3,"character":28}}}"#;
    writer.write_all(frame(def_req2).as_bytes()).await.unwrap();
    let def_resp2 = read_until_id(&mut reader, "\"id\":4", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp2.contains(r#""line":2"#),
        "linked bareword sibling call must resolve to foo's declaration on line 2: {def_resp2}",
    );
    let hover_req2 = r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///link2.tcl"},"position":{"line":3,"character":28}}}"#;
    writer
        .write_all(frame(hover_req2).as_bytes())
        .await
        .unwrap();
    let hover_resp2 = read_until_id(&mut reader, "\"id\":5", 8)
        .await
        .expect("hover response");
    assert!(
        hover_resp2.contains("**method**") && hover_resp2.contains("Widget::foo"),
        "linked bareword sibling call must resolve hover to foo's declaration: {hover_resp2}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TP — issue #923 idx 9: `set VAR [interp create ...]` never bound `VAR` to
/// the interpreter it created, so a later dynamic `interp alias $VAR name {}
/// target` / `interp eval $VAR { ... }` pair abstained outright — tclsh9.0-
/// verified this idiom (a `-safe` sandbox alias-and-eval, the shape
/// tcllib's doctools.tcl uses) actually runs and prints 42; the LSP saw a
/// spurious "unknown command" and returned no definition location for the
/// aliased call at all.
#[tokio::test]
async fn dynamic_interp_handle_alias_definition_follows_the_tracked_binding_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///dyninterp.tcl",
        "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset s [interp create -safe]\ninterp alias $s greet {} ::app::Helper\ninterp eval $s { greet }\n",
    )
    .await;

    let diags = published_codes(&mut reader, "file:///dyninterp.tcl").await;
    assert!(
        !diags.contains("W123") && !diags.contains("W140"),
        "tracked binding must not leave a spurious unknown-command or \
         never-created-interpreter warning: {diags}",
    );

    // `greet` inside `interp eval $s { greet }` (line 4, character 17).
    let def_req = r#"{"jsonrpc":"2.0","id":7,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///dyninterp.tcl"},"position":{"line":4,"character":17}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":7", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp.contains(r#""line":1"#),
        "greet must resolve through the tracked $s binding to ::app::Helper \
         on line 1, following the cross-domain alias: {def_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TP — issue #923 idx 120: a class's own bound command name
/// (`ActiveRecord find ...` calling its own `classmethod`, and the same
/// call inherited by a non-overriding subclass's own command,
/// `Table find ...`) never resolved — `receiver_instance_class` only ever
/// recognised a `$var`/created-instance-command receiver, never a bare
/// word naming a class directly. tclsh9.0-verified: both calls print
/// `::ActiveRecord called with arguments: foo bar`.
#[tokio::test]
async fn classmethod_dispatch_on_class_and_inheriting_subclass_resolves_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///activerecord.tcl",
        "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n",
    )
    .await;

    let diags = published_codes(&mut reader, "file:///activerecord.tcl").await;
    assert!(
        !diags.contains("W123"),
        "unexpected unknown-command: {diags}"
    );

    // `find` in `Table find foo bar` (line 6, character 6) — inherited via
    // ooutil-style classmethod propagation through the superclass MRO.
    let def_req = r#"{"jsonrpc":"2.0","id":8,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///activerecord.tcl"},"position":{"line":6,"character":6}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":8", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp.contains(r#""line":1"#),
        "Table find must resolve to ActiveRecord's classmethod find on line 1: {def_resp}",
    );

    // `find` in `ActiveRecord find foo bar` (line 7, character 13) — the
    // declaring class's own call.
    let def_req2 = r#"{"jsonrpc":"2.0","id":9,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///activerecord.tcl"},"position":{"line":7,"character":13}}}"#;
    writer.write_all(frame(def_req2).as_bytes()).await.unwrap();
    let def_resp2 = read_until_id(&mut reader, "\"id\":9", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp2.contains(r#""line":1"#),
        "ActiveRecord find must resolve to its own classmethod find on line 1: {def_resp2}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TP — issue #923 idx 120, the `self method` half of the finding: stock
/// `TclOO`'s own spelling of a class-level method (`self method NAME ARGS
/// BODY`) had no `apply_oo_subcommand` arm at all, so it was invisible to
/// `class_methods`, its body was never walked, and — even once
/// recognised — is NOT inherited by a subclass with no override the way
/// `ooutil`'s `classmethod` is (tclsh9.0-verified: `Gadget make` raises
/// `unknown method "make"`).
#[tokio::test]
async fn self_method_dispatch_resolves_but_is_not_inherited_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///widget.tcl",
        "oo::class create Widget {\n    self method make {n} {\n        string length a b c d\n        return \"made $n\"\n    }\n}\noo::class create Gadget {\n    superclass Widget\n}\nWidget make gadget\nGadget make gadget\n",
    )
    .await;

    let diags = published_codes(&mut reader, "file:///widget.tcl").await;
    assert!(
        diags.contains("E003"),
        "the wrong-arity call inside the self-method body must now be walked: {diags}",
    );

    // `make` in `Widget make gadget` (line 9, character 7) resolves to the
    // declaration.
    let def_req = r#"{"jsonrpc":"2.0","id":10,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///widget.tcl"},"position":{"line":9,"character":7}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":10", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp.contains(r#""line":1"#),
        "Widget make must resolve to its own self-method declaration on line 1: {def_resp}",
    );

    // `make` in `Gadget make gadget` (line 10, character 7) must NOT
    // resolve — a plain `self method` is not inherited.
    let def_req2 = r#"{"jsonrpc":"2.0","id":11,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///widget.tcl"},"position":{"line":10,"character":7}}}"#;
    writer.write_all(frame(def_req2).as_bytes()).await.unwrap();
    let def_resp2 = read_until_id(&mut reader, "\"id\":11", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp2.contains(r#""result":[]"#) || def_resp2.contains(r#""result":null"#),
        "Gadget make must NOT resolve — self method is not inherited: {def_resp2}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}

/// TP — issue #923 idx 116: `apply {{params} body ns}` runs `body` in
/// `ns`, not the namespace the `apply` call is lexically written inside.
/// tclsh9.0/8.6-verified: `real::cleanup` runs, printing "real done".
/// Before the fix, the bareword `cleanup` inside the apply body resolved
/// to whichever same-named proc happened to be lexically nearest
/// (`lexical::cleanup`) — a `Scope` subtree `handle_apply_command` built
/// for `::real` was structurally unreachable by the span-containment walk.
#[tokio::test]
async fn apply_namespace_override_resolves_bareword_to_the_lambdas_own_namespace_e2e() {
    let (mut reader, mut writer, server) = start_session().await;
    did_open(
        &mut writer,
        "file:///apply_ns.tcl",
        "namespace eval real {\n    proc cleanup {tag} { puts \"real $tag\" }\n}\nnamespace eval lexical {\n    proc cleanup {tag} { puts \"lexical $tag\" }\n}\napply {{} {cleanup done} ::real}\n",
    )
    .await;

    let diags = published_codes(&mut reader, "file:///apply_ns.tcl").await;
    assert!(
        !diags.contains("W123"),
        "unexpected unknown-command: {diags}"
    );

    // `cleanup` inside the apply body (line 6, character 11).
    let def_req = r#"{"jsonrpc":"2.0","id":12,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///apply_ns.tcl"},"position":{"line":6,"character":11}}}"#;
    writer.write_all(frame(def_req).as_bytes()).await.unwrap();
    let def_resp = read_until_id(&mut reader, "\"id\":12", 8)
        .await
        .expect("definition response");
    assert!(
        def_resp.contains(r#""line":1"#),
        "cleanup must resolve to real::cleanup on line 1, not lexical::cleanup on line 4: {def_resp}",
    );

    writer
        .write_all(frame(r#"{"jsonrpc":"2.0","method":"exit","params":null}"#).as_bytes())
        .await
        .unwrap();
    drop(writer);
    server.abort();
}
