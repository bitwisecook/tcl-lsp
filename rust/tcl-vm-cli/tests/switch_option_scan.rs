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

//! A `switch` subject the compiler cannot know: 8.4 scans it as an option
//! word, so a value beginning with `-` is `bad option`; later releases match
//! it. Compiled code keeps the release's scan (#2310).

use std::path::PathBuf;
use std::process::Command;

const SCRIPT: &str = "\
proc f {} {return -x}
puts [catch {switch [f] {-x {puts m} default {puts d}}} r]:$r
proc g {} {switch [f] {-x {return m} default {return d}}}
puts [catch g r]:$r
set v -exact
puts [catch {switch $v {-exact {puts m} default {puts d}}} r]:$r
proc h {} {switch -- [f] {-x {return m} default {return d}}}
puts [catch h r]:$r
";

/// Measured on tclsh 8.4.20, 8.5.19, 8.6.18 and 9.0.4.
fn expected(release: &str) -> &'static str {
    if release == "8.4" {
        "1:bad option \"-x\": must be -exact, -glob, -regexp, or --\n\
         1:bad option \"-x\": must be -exact, -glob, -regexp, or --\n\
         1:bad option \"-exact {puts m} default {puts d}\": must be -exact, -glob, -regexp, or --\n\
         0:m\n"
    } else {
        "m\n0:\n0:m\nm\n0:\n0:m\n"
    }
}

fn script() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-switch-scan-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    let path = dir.join("switch_scan.tcl");
    std::fs::write(&path, SCRIPT).expect("writes the script");
    path
}

fn stdout(command: &mut Command) -> Option<String> {
    let output = command.env("LC_ALL", "C.UTF-8").output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn an_unknown_switch_subject_is_scanned_as_each_release_scans_it() {
    let path = script();
    for release in ["8.4", "8.5", "8.6", "9.0"] {
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
