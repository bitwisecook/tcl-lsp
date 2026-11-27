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

//! Public JSON conversion to `SslicTcl` through the data CLI.

use std::process::{Command, Stdio};

#[test]
fn testssl_conversion_emits_loadable_sslictcl() {
    let sample = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/testssl-sample.json"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sslictcl-data"))
        .args(["testssl-to-dsl", sample])
        .stdin(Stdio::null())
        .output()
        .expect("spawn the binary");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let source = String::from_utf8(output.stdout).expect("SslicTcl text");
    assert_ne!(source, "");
    tcl_sslictcl::load(&source).expect("conversion emits valid SslicTcl");
}
