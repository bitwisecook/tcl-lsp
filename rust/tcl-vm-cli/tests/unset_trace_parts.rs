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

//! A compiled `unset` of a qualified array element holds the name as two
//! parts, and its unset trace reports the array part as name1, as C's
//! `unsetArrayStk` does; a directly evaluated one reports the spelling the
//! release's own lookup reports (#1946).

use std::path::PathBuf;
use std::process::Command;

const SCRIPT: &str = "\
proc tr {args} { lappend ::log [lrange $args 0 1] }
set a(k) 1; trace add variable a unset tr
proc t5 {} { unset ::a(k) }
set ::log {}; t5; puts \"proc: $::log\"
set a(k) 1; set ::log {}; unset ::a(k); puts \"top: $::log\"
set a(k) 1; set ::log {}; namespace eval n { unset ::a(k) }; puts \"ns: $::log\"
proc t6 {} { set ::a(k) 1; set n ::a; unset ${n}(k) }
set ::log {}; t6; puts \"dyn: $::log\"
";

/// Measured on tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
fn expected(release: &str) -> &'static str {
    if release.starts_with('8') {
        "proc: {::a k}\ntop: {::a k}\nns: {::a k}\ndyn: {::a k}\n"
    } else {
        "proc: {::a k}\ntop: {::a(k) k}\nns: {::a k}\ndyn: {::a(k) k}\n"
    }
}

fn script() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-unset-parts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    let path = dir.join("unset_parts.tcl");
    std::fs::write(&path, SCRIPT).expect("writes the script");
    path
}

fn stdout(command: &mut Command) -> Option<String> {
    let output = command.env("LC_ALL", "C.UTF-8").output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn a_compiled_qualified_element_unset_reports_its_array_part() {
    let path = script();
    for release in ["8.5", "8.6", "9.0", "9.1"] {
        let vm = stdout(
            Command::new(env!("CARGO_BIN_EXE_tclvm"))
                .args(["--tcl-version", release])
                .arg(&path),
        )
        .expect("run tclvm");
        assert_eq!(vm, expected(release), "tclvm {release}");
        if let Some(tclsh) = stdout(Command::new(format!("tclsh{release}")).arg(&path)) {
            assert_eq!(tclsh, expected(release), "tclsh{release}");
        }
    }
    let _ = std::fs::remove_dir_all(path.parent().expect("scratch directory"));
}
