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

use std::process::{Command, Stdio};

#[test]
fn the_binary_installs_its_surfaces_before_reading_the_registry() {
    let findings = std::env::temp_dir().join(format!("tcl-fuzz-installs-{}", std::process::id()));
    let findings = findings.to_str().expect("a UTF-8 temporary path");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl-fuzz"))
        .args([
            "--findings",
            findings,
            "bpf-diff",
            "--iterations",
            "1",
            "--seed",
            "1",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("spawn the binary");
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
    assert!(output.status.success(), "{stderr}");
}
