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
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn the_server_installs_its_surfaces_before_analysing_a_source() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tcl-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the server");
    let mut stdin = child.stdin.take().expect("the server's stdin");
    for line in [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"installs-surfaces","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"analyze","arguments":{"source":"set x [llength {a b}]\nputs $x\n"}}}"#,
    ] {
        writeln!(stdin, "{line}").expect("write a request");
    }
    stdin.flush().expect("flush the requests");
    let stdout = BufReader::new(child.stdout.take().expect("the server's stdout"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in stdout.lines().map_while(Result::ok) {
            if line.contains(r#""id":2"#) {
                let _ = sender.send(line);
            }
        }
    });
    let answer = receiver.recv_timeout(Duration::from_secs(120)).ok();
    drop(stdin);
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
    let answer = answer.unwrap_or_else(|| panic!("no answer to the tool call: {stderr}"));
    assert!(answer.contains("diagnostics"), "{answer}");
}
