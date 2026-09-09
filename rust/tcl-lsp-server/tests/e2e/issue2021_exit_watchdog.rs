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

//! End-to-end coverage for the exit watchdog (issue #2021): once a session is
//! over — stdin reaching EOF, or the `exit` notification arriving — the
//! server process must terminate within a short bounded grace even while a
//! handler future is still running (here, the `initialized` workspace scan
//! that started it all).
//!
//! `write_slow_workspace` makes the scan provably slow two ways at once,
//! so this stays a real regression test regardless of which fix has landed
//! in `tcl-compiler` when it runs: an exponential-blowup Tcl file — the
//! `switch_empty5` shape from `repro/issue-2021/phi/gen.py`, generated inline
//! here rather than shelling out to python — plus several hundred files of
//! plain, ordinary procs, so even a server whose analyser no longer chokes on
//! the pathological shape still has a genuinely multi-second scan ahead of
//! it. Either alone would do; both together mean this test cannot quietly
//! start passing "trivially" the moment the exponential case is fixed.
//!
//! Drives the real binary directly with hand-rolled JSON-RPC framing rather
//! than through `common::Lsp`: `Lsp::drop` sends `shutdown`/`exit` and
//! force-kills the child after 50ms, which is exactly the behaviour these
//! tests need *not* to happen in order to observe the watchdog rather than
//! the harness's own cleanup.

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write as _};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// How long the watchdog's grace period is set to for these tests — far
/// below the 3s production default, so a healthy pass still finishes in a
/// couple of seconds rather than several.
const GRACE_MS: u64 = 500;

/// The outer bound `child.try_wait()` is polled against: comfortably above
/// [`GRACE_MS`] plus the time the workspace scan and process teardown
/// legitimately need, while still catching a watchdog that does not fire at
/// all (an orphan would sit at this bound forever otherwise).
const EXIT_BOUND: Duration = Duration::from_secs(10);

// ------------------------------------------------------------ workspace generation

/// The exponential-shape reproducer from `repro/issue-2021/phi/gen.py`'s
/// `switch_empty5(n)` variant with its default flags (`init=true, read=true,
/// proc=false`): `n` chained `switch` statements, each with one live arm and
/// several empty arms, which is what fed the analyser's phi-merge worst case
/// in issue #2021. Kept in sync with `gen.py` by construction rather than by
/// hand — see that file for the Python original this mirrors line for line.
fn switch_empty5_reproducer(n: usize) -> String {
    let mut out = String::new();
    out.push_str("set q 0\nset r 0\nset items [lindex $argv 0]\n");
    for i in 0..n {
        let _ = write!(
            out,
            "set c{i} [lindex $argv {i}]\nset d{i} [lindex $argv {i}]\n\
             set e{i} [lindex $argv {i}]\nset v{i} [lindex $argv {i}]\n"
        );
    }
    out.push_str("set x 0\n");
    for i in 0..n {
        let _ = write!(out, "switch -- $v{i} {{\n    a {{ set x {i} }}\n");
        for arm in 0..4 {
            let _ = writeln!(out, "    b{arm} {{ }}");
        }
        out.push_str("}\n");
    }
    out.push_str("puts $x\n");
    out
}

/// One file of plain, ordinary procs — nothing pathological, just bulk: 330
/// six-line procs is ~1980 lines, close to the "2000 lines of ordinary procs"
/// this is modelled on.
fn ordinary_procs_file(file_index: usize) -> String {
    const PROCS_PER_FILE: usize = 330;
    let mut out = String::with_capacity(PROCS_PER_FILE * 90);
    for i in 0..PROCS_PER_FILE {
        let _ = write!(
            out,
            "proc ordinary_{file_index}_{i} {{a b c}} {{\n\
            \x20   set total [expr {{$a + $b + $c}}]\n\
            \x20   if {{$total > {i}}} {{\n\
            \x20       return $total\n\
            \x20   }}\n\
            \x20   return [expr {{-$total}}]\n\
            }}\n"
        );
    }
    out
}

/// Write a workspace at `root` whose startup scan takes several seconds
/// regardless of the exponential-blowup analyser fix — see the module docs.
fn write_slow_workspace(root: &Path) {
    const ORDINARY_FILES: usize = 300;
    std::fs::create_dir_all(root).expect("mkdir workspace root");
    std::fs::write(root.join("exponential.tcl"), switch_empty5_reproducer(20))
        .expect("write exponential reproducer");
    for file_index in 0..ORDINARY_FILES {
        std::fs::write(
            root.join(format!("ordinary_{file_index}.tcl")),
            ordinary_procs_file(file_index),
        )
        .expect("write ordinary workspace file");
    }
}

// ------------------------------------------------------------ raw JSON-RPC framing

fn write_message(stdin: &mut ChildStdin, payload: &Value) {
    let body = serde_json::to_vec(payload).expect("serialise JSON-RPC");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
    stdin.write_all(&body).expect("write body");
    stdin.flush().expect("flush");
}

fn read_message(reader: &mut impl BufRead) -> Value {
    let mut content_length = 0usize;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).expect("read header line");
        assert!(n > 0, "server stdout closed while reading a header");
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        saw_header = true;
        if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().expect("parse Content-Length");
        }
    }
    assert!(saw_header, "server stdout closed with no header at all");
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).expect("read message body");
    serde_json::from_slice(&body).expect("parse JSON-RPC body")
}

// ------------------------------------------------------------ process lifecycle

/// Resolve the real server binary the same way `common::Lsp` does: prefer
/// nextest's archived runtime path, fall back to Cargo's compile-time one.
fn server_binary() -> std::ffi::OsString {
    std::env::var_os("NEXTEST_BIN_EXE_tcl-lsp-server")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_tcl-lsp-server").into())
}

/// Spawn the server with its own isolated XDG dirs (so a developer's local
/// config can't poison the defaults, same reasoning as `common::Lsp`) and
/// [`GRACE_MS`] as its watchdog grace period. Drains stderr on a background
/// thread purely so the server's stderr pipe never fills and blocks it —
/// unrelated to what this test asserts on.
fn spawn_server(label: &str) -> Child {
    let xdg_root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-watchdog-xdg-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(xdg_root.join("config")).expect("mk xdg config");
    std::fs::create_dir_all(xdg_root.join("cache")).expect("mk xdg cache");

    let mut child = Command::new(server_binary())
        .env("XDG_CONFIG_HOME", xdg_root.join("config"))
        .env("XDG_CACHE_HOME", xdg_root.join("cache"))
        .env("TCL_LSP_EXIT_GRACE_MS", GRACE_MS.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tcl-lsp-server");

    let stderr = child.stderr.take().expect("child stderr");
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut discard = String::new();
        let _ = Read::read_to_string(&mut reader, &mut discard);
    });

    child
}

/// Send `initialize` + `initialized` against `root`, returning the still-open
/// stdin/stdout handles so the caller controls exactly what happens next.
fn handshake(child: &mut Child, root: &Path) -> (ChildStdin, BufReader<std::process::ChildStdout>) {
    let mut stdin = child.stdin.take().expect("child stdin");
    let mut reader = BufReader::new(child.stdout.take().expect("child stdout"));

    let root_uri = format!("file://{}", root.to_string_lossy());
    write_message(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "workspaceFolders": [{ "uri": root_uri, "name": "e2e" }],
                "capabilities": {},
                "clientInfo": { "name": "tcl-lsp-e2e-watchdog", "version": "1.0" },
            }
        }),
    );
    let response = read_message(&mut reader);
    assert!(
        response.get("result").is_some(),
        "initialize failed: {response}"
    );

    write_message(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
    );

    (stdin, reader)
}

/// Poll until the child exits (or panic past [`EXIT_BOUND`]), returning its
/// exit status. `try_wait` reaps the process the moment it observes the exit,
/// so no explicit `wait()` is needed afterwards.
fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + EXIT_BOUND;
    loop {
        if let Some(status) = child.try_wait().expect("poll child status") {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "server did not exit within {EXIT_BOUND:?} of the session ending \
             (watchdog grace was {GRACE_MS}ms) — it is orphaned, exactly what issue #2021 was"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ------------------------------------------------------------ tests

/// stdin EOF mid-scan, with no `shutdown`/`exit` ever sent, must still force
/// the process to exit — and per the LSP spec's `exit` contract the watchdog
/// applies uniformly, an unclean session exits `1`.
///
/// The exit code assertion is only meaningful because [`write_slow_workspace`]
/// guarantees the `initialized` scan is still the in-flight handler future
/// when stdin closes: that is what stops `Server::serve` from returning
/// through its own ordinary path (which exits `0` unconditionally) before the
/// watchdog's grace elapses, so it is the watchdog — and its `shutdown_seen`
/// check — that decides the outcome here, not a race with the normal exit.
#[test]
fn eof_mid_scan_without_shutdown_forces_exit_within_grace() {
    let workspace = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-watchdog-ws-eof-{}",
        std::process::id()
    ));
    write_slow_workspace(&workspace);

    let mut child = spawn_server("eof");
    let (stdin, reader) = handshake(&mut child, &workspace);

    // Let the workspace scan get underway.
    std::thread::sleep(Duration::from_secs(1));

    // Session over: stdin EOF, no `shutdown`/`exit` — and stop reading
    // stdout too, matching a genuinely orphaned client (the extension host
    // that exited without a word in issue #2021's original report).
    drop(stdin);
    drop(reader);

    let status = wait_for_exit(&mut child);
    assert_eq!(
        status.code(),
        Some(1),
        "an unclean session (EOF, no shutdown) must exit 1 per the LSP spec's \
         `exit` notification, got {status:?}"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

/// `shutdown` followed by `exit`, sent mid-scan, must also force a prompt
/// exit — this time `0`, because `shutdown` was observed first. Stdin is
/// deliberately left open and stdout deliberately unread from this point:
/// the watchdog must fire from the `exit` notification alone, the same as a
/// real client that considers the subprocess gone the moment it has sent
/// `exit` and stops driving it any further either way.
#[test]
fn shutdown_then_exit_mid_scan_forces_clean_exit_within_grace() {
    let workspace = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-watchdog-ws-exit-{}",
        std::process::id()
    ));
    write_slow_workspace(&workspace);

    let mut child = spawn_server("exit");
    let (mut stdin, reader) = handshake(&mut child, &workspace);

    // Let the workspace scan get underway.
    std::thread::sleep(Duration::from_secs(1));

    write_message(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "shutdown", "params": null }),
    );
    write_message(
        &mut stdin,
        &json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );
    drop(reader);

    let status = wait_for_exit(&mut child);
    assert_eq!(
        status.code(),
        Some(0),
        "shutdown followed by exit must exit 0, got {status:?}"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}
