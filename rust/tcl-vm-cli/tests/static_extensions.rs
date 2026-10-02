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

//! The host's opt-in `load` of the shipping `tclvm` driver, through the
//! process: the flag gives a script a `load` over the extensions the build
//! links in, nothing else does, and a build without the feature says so.

use std::process::{Command, Output};

fn tclvm(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .args(args)
        .output()
        .expect("run tclvm")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[cfg(all(feature = "static-extensions", not(windows)))]
#[test]
fn the_flag_gives_a_script_a_load_over_the_linked_extension() {
    let output = tclvm(&[
        "--static-extensions",
        "-c",
        "load {} Pkga; puts [pkga_eq a a]; puts [pkga_calc add 2 3]; \
         puts [catch {load libnosuch.so} m]; puts $m",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "1\n5\n1\ncouldn't load file \"libnosuch.so\": no extension with the prefix \"Nosuch\" \
         is linked into this program\n"
    );
}

#[cfg(feature = "static-extensions")]
#[test]
fn without_the_flag_there_is_no_load_even_in_a_build_that_links_the_extension() {
    let output = tclvm(&["-c", "load {} Pkga"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("invalid command name \"load\""),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_usage_text_names_the_flag_and_is_shown_whatever_the_build() {
    let output = tclvm(&["--static-extensions", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("--static-extensions"),
        "{}",
        stderr(&output)
    );
}

#[cfg(not(feature = "static-extensions"))]
#[test]
fn the_flag_needs_a_build_with_the_feature() {
    let output = tclvm(&["--static-extensions", "-c", "puts ran"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty(), "the script did not run");
    assert!(
        stderr(&output).contains("`static-extensions` feature"),
        "{}",
        stderr(&output)
    );
}
