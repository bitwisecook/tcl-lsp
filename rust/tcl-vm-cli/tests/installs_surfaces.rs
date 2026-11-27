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
//! The binary runs without the variable through which cargo-run processes
//! install the shipped list on their own, so a binary that read the registry
//! first would panic with the registry's not-installed message.

use std::process::{Command, Stdio};

#[test]
fn the_binary_installs_its_surfaces_before_reading_the_registry() {
    let dir = std::env::temp_dir().join(format!("tclvm-installs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the probe directory");
    let probe = dir.join("probe.tcl");
    std::fs::write(&probe, "puts [llength {a b}]\n").expect("write the probe");
    let probe = probe.to_str().expect("a UTF-8 temporary path");
    let output = Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .env_remove("TCL_REGISTRY_TEST_INSTALL")
        .args([probe])
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
}
