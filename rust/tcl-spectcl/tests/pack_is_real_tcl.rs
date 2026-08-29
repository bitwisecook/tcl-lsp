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

//! A `.tclspec` pack is a **Tcl script**, and the arbiter of that is a real
//! Tcl interpreter, not our own parser.
//!
//! Design E's whole claim is that a pack is a program the VM executes, with
//! the registration vocabulary installed as host commands. If a shipped pack
//! were only *nearly* Tcl — a stray unbalanced brace inside a comment, a `$`
//! or `[` our segmenter happens to tolerate — the claim would be false and
//! our own agreement with ourselves would not notice.
//!
//! So: hand every shipped pack to `tclsh9.0` with the vocabulary swallowed by
//! `unknown`, and require it to source cleanly. What survives is a parse
//! judgement from an interpreter that knows nothing about SpecTcl.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `unknown` swallows every vocabulary word, so the only thing left that can
/// fail is Tcl's own reading of the file.
const DRIVER: &str = r#"
proc unknown {args} { return "" }
set failures {}
foreach path $argv {
    if {[catch {uplevel #0 [list source $path]} err]} {
        lappend failures "$path: $err"
    }
}
puts [join $failures "\n"]
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn every_shipped_pack_is_a_tcl_script_a_real_tclsh_accepts() {
    let root = repo_root();
    let packs = tcl_spectcl::golden::shipped_packs(&root);
    assert!(
        packs.len() >= 24,
        "the inventory must cover the shipped packs"
    );

    let mut child = Command::new("tclsh9.0")
        .arg("-")
        .args(packs.iter().map(|p| p.as_os_str()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tclsh9.0 oracle available");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(DRIVER.as_bytes())
        .expect("write the driver");
    let out = child.wait_with_output().expect("tclsh9.0 runs");
    let failures = String::from_utf8_lossy(&out.stdout);
    assert!(
        failures.trim().is_empty(),
        "a real tclsh refuses to source these packs:\n{failures}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
