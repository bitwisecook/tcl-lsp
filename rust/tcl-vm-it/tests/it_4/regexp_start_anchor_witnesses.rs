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

//! `regexp -start` matches against the subject from the offset on, as C Tcl's
//! `Tcl_RegExpExecObj` does: `\A` and word-boundary context start there, and
//! only `^` honours `REG_NOTBOL`. tcllib 2.0's `string::token` lexer is built
//! on `\A` at successive offsets, so `string::token::shell` failed without it.

use crate::common;

use tcl_dialect::TclVersion;

const MINIMAL: &str = r"puts [list [regexp -start 1 {\Ab} abc] [regexp -start 1 {^b} abc]]";

#[test]
fn backslash_a_matches_at_the_start_offset_and_caret_does_not() {
    for oracle in tcl_test_support::required_tclshs(&TclVersion::ALL)
        .expect("matching original C interpreters")
    {
        assert_eq!(
            common::oracle_output(&oracle.path, MINIMAL),
            "1 0",
            "{}: original",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(MINIMAL, oracle.version.dialect_profile_name()),
            "1 0",
            "{}: compiled",
            oracle.patchlevel
        );
    }
}

/// tcllib 2.0's module tree: `TCLLIB_2_0_DIR` or the checkout's `tmp/tcllib-2.0`.
fn tcllib_modules() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
        std::path::PathBuf::from,
    );
    let modules = root.join("modules");
    modules
        .join("string/token_shell.tcl")
        .is_file()
        .then_some(modules)
}

const SHELL_TOKENS: &str = "\
0 {cp {a b} c} {}
1 {Unexpected character '\"' at offset 5} {STRING TOKEN SHELL BAD SYNTAX CHAR}
1 {Expected whitespace, got start of word at offset 1} {STRING TOKEN SHELL BAD SYNTAX WORD}";

#[test]
fn tcllib_string_token_shell_splits_as_tclsh_does() {
    let Some(modules) = tcllib_modules() else {
        eprintln!(
            "[regexp_start_anchor] skipped: no tcllib 2.0 under tmp/ (fetch it, or set TCLLIB_2_0_DIR)"
        );
        return;
    };
    let source = format!(
        r#"foreach module {{string fileutil cmdline}} {{
    set dir [file join {{{}}} $module]
    source [file join $dir pkgIndex.tcl]
}}
package require string::token::shell
foreach text {{{{cp 'a b' c}} {{echo "x}} {{a'b'}}}} {{
    set failed [catch {{string::token::shell $text}} result options]
    puts [list $failed $result [expr {{$failed ? [dict get $options -errorcode] : {{}}}}]]
}}
"#,
        modules.display()
    );
    let versions = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    for oracle in
        tcl_test_support::required_tclshs(&versions).expect("matching original C interpreters")
    {
        assert_eq!(
            common::oracle_output(&oracle.path, &source),
            SHELL_TOKENS,
            "{}: original",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(&source, oracle.version.dialect_profile_name()),
            SHELL_TOKENS,
            "{}: compiled",
            oracle.patchlevel
        );
    }
}
