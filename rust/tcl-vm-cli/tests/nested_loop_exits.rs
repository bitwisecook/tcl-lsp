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

//! A `break` or `continue` in an inner loop leaves only that loop, whether
//! the loops are compiled at the top level or in a procedure (#2160).

use std::path::PathBuf;
use std::process::Command;

const SCRIPT: &str = "\
set n 0
while {$n < 2} { while {1} { break }; incr n }
puts top=$n
proc w {} { set n 0; while {$n < 2} { while {1} { incr n; break } }; return $n }
puts w=[w]
proc f {} { set out {}; foreach x {1 2} { foreach y {a b} { break }; lappend out $x }; return $out }
puts f=[f]
proc r {} { set n 0; for {} {$n < 2} {} { for {set k 0} {$k < 3} {incr k} { continue }; incr n }; return $n }
puts r=[r]
proc c {} { set n 0; while {$n < 2} { set k 0; while {$k < 1} { incr k; continue }; incr n }; return $n }
puts c=[c]
proc d {} { set out {}; dict for {k v} {a 1 b 2} { foreach y {1 2} { break }; lappend out $k }; return $out }
puts d=[d]
";

/// Measured on tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
const EXPECTED: &str = "top=2\nw=2\nf=1 2\nr=2\nc=2\nd=a b\n";

fn script() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-nested-loops-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    let path = dir.join("nested_loops.tcl");
    std::fs::write(&path, SCRIPT).expect("writes the script");
    path
}

fn stdout(command: &mut Command) -> Option<String> {
    let output = command.env("LC_ALL", "C.UTF-8").output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn an_inner_loop_exit_leaves_only_the_inner_loop() {
    let path = script();
    for release in ["8.5", "8.6", "9.0", "9.1"] {
        let vm = stdout(
            Command::new(env!("CARGO_BIN_EXE_tclvm"))
                .args(["--tcl-version", release])
                .arg(&path),
        )
        .expect("run tclvm");
        assert_eq!(vm, EXPECTED, "tclvm {release}");
        if let Some(tclsh) = stdout(Command::new(format!("tclsh{release}")).arg(&path)) {
            assert_eq!(tclsh, EXPECTED, "tclsh{release}");
        }
    }
    let _ = std::fs::remove_dir_all(path.parent().expect("scratch directory"));
}
