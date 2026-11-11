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

//! `INT64_MIN`'s string rep: 8.4 and 8.5 spell a computed one `-8`, and every
//! later reading of the value reads that spelling (#2485).

use std::path::PathBuf;
use std::process::Command;

const SCRIPT: &str = "\
set m [expr {-9223372036854775807-1}]
puts $m
puts [expr {-9223372036854775808}]
puts [expr {wide(-9223372036854775807-1)}]
puts [expr {~9223372036854775807}]
puts [string length $m]
puts [format %d $m]
puts [format %05d $m]
puts [format %.3d $m]
puts [catch {format %lld $m} r]$r
puts [format %u $m]
puts [expr {9223372036854775808}]
puts [expr {18446744073709551615}]
puts [list $m [expr {$m + 0}]]
";

const MIN: &str = "-9223372036854775808";

/// Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
fn expected(release: &str) -> String {
    let lines: Vec<String> = match release {
        "8.4" => vec![
            "-8",
            "-8",
            "-8",
            "-8",
            "2",
            "-8",
            "-0008",
            "-008",
            "1bad field specifier \"l\"",
            "18446744073709551608",
            "-8",
            "-1",
            "-8 -8",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "8.5" => vec![
            "-8",
            "-8",
            "-8",
            "-8",
            "2",
            "-8",
            "-0008",
            "-008",
            "0-8",
            "18446744073709551608",
            "9223372036854775808",
            "18446744073709551615",
            "-8 -8",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "8.6" => vec![
            MIN.to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            "20".to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            format!("0{MIN}"),
            "9223372036854775808".to_owned(),
            "9223372036854775808".to_owned(),
            "18446744073709551615".to_owned(),
            format!("{MIN} {MIN}"),
        ],
        _ => vec![
            MIN.to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            MIN.to_owned(),
            "20".to_owned(),
            "0".to_owned(),
            "00000".to_owned(),
            "000".to_owned(),
            format!("0{MIN}"),
            "0".to_owned(),
            "9223372036854775808".to_owned(),
            "18446744073709551615".to_owned(),
            format!("{MIN} {MIN}"),
        ],
    };
    lines.join("\n") + "\n"
}

fn script() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-wide-min-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    let path = dir.join("wide_min.tcl");
    std::fs::write(&path, SCRIPT).expect("writes the script");
    path
}

fn stdout(command: &mut Command) -> Option<String> {
    let output = command.env("LC_ALL", "C.UTF-8").output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn int64_min_is_spelled_as_each_release_spells_it() {
    let path = script();
    for release in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
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
