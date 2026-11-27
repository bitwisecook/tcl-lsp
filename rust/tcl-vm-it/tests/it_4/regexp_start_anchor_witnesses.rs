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

/// Past the end C matches the empty remainder and positions it at the
/// caller's offset; a capture there is the empty string.
const PAST_THE_END: &str = r"puts [regexp -start 10 -indices -inline {\A} abc]
puts [list [regexp -start 10 {\A()} abc m g] $m $g]
puts [regsub -start 10 -all {\A} abc X]";

/// `-start end` is the string length itself, one past the last character.
const START_END: &str = r"puts [regexp -start end -indices -inline {\A} abc]
puts [regexp -start end-1 -inline {\A.} abc]
puts [regsub -start end a aaa b]";

fn matches_original(source: &str, expected: &str, versions: &[TclVersion]) {
    for oracle in
        tcl_test_support::required_tclshs(versions).expect("matching original C interpreters")
    {
        assert_eq!(
            common::oracle_output(&oracle.path, source),
            expected,
            "{}: original",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(source, oracle.version.dialect_profile_name()),
            expected,
            "{}: compiled",
            oracle.patchlevel
        );
    }
}

#[test]
fn backslash_a_matches_at_the_start_offset_and_caret_does_not() {
    matches_original(MINIMAL, "1 0", &TclVersion::ALL);
}

#[test]
fn a_start_past_the_end_reports_the_callers_offset() {
    matches_original(PAST_THE_END, "{10 9}\n1 {} {}\nabc", &TclVersion::ALL);
}

#[test]
fn start_end_is_the_string_length() {
    let versions = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    matches_original(START_END, "{3 2}\nc\naaa", &versions);
}

/// `^` at a restart offset just past a newline: 8.4's `regexp` suppresses
/// it at every positive offset, 8.5 on only where the previous character is
/// not a newline; `regsub` reads the previous character on every release.
const RESTART_AFTER_NEWLINE: &str = r#"puts [list \
    [regexp -start 3 -line -inline {^.} "ab\ncd"] \
    [regexp -all -inline -line {^.|\n} "a\nb"] \
    [regsub -start 3 -line {^.} "ab\ncd" X]]"#;

#[test]
fn regexp_restart_after_a_newline_follows_the_release() {
    matches_original(
        RESTART_AFTER_NEWLINE,
        "{} {a {\n}} {ab\nXd}",
        &[TclVersion::V8_4],
    );
    let versions = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    matches_original(RESTART_AFTER_NEWLINE, "c {a {\n} b} {ab\nXd}", &versions);
}

/// tcllib 2.0's `string::token::text` / `chomp` over the `string::token::shell`
/// lexer, inlined so the witness runs where no tcllib tree is fetched (CI).
const TOKEN_LEXER: &str = r#"proc chomp {map sv text rv} {
    upvar 1 $sv start $rv result
    if {$start >= [string length $text]} {return 2}
    foreach {pattern label} $map {
        if {![regexp -start $start -indices -- \\A($pattern) $text -> range]} continue
        lappend result [list $label {*}$range]
        set start [expr {[lindex $range 1] + 1}]
        return 1
    }
    return 0
}
proc lex {text} {
    set map [list {\s+} WSPACE {'[^']*'} S:QUOTED \
        "\"(\[^\"\]|(\\\\\")|(\\\\\\\\))*\"" D:QUOTED \
        "((\[^ \\s'\"\])|(\\\\\")|(\\\\\\\\))+" PLAIN {.*} ERROR]
    set start 0
    set result {}
    while {[set status [chomp $map start $text result]] == 1} {}
    if {$status == 0} {return [list BAD $start]}
    return $result
}
foreach text {{cp 'a b' c} {echo "x} {a'b'}} {puts [lex $text]}"#;

const LEXER_TOKENS: &str = "\
{PLAIN 0 1} {WSPACE 2 2} {S:QUOTED 3 7} {WSPACE 8 8} {PLAIN 9 9}
{PLAIN 0 3} {WSPACE 4 4} {ERROR 5 6}
{PLAIN 0 0} {S:QUOTED 1 3}";

#[test]
fn tcllib_string_token_lexer_inlined() {
    let versions = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    matches_original(TOKEN_LEXER, LEXER_TOKENS, &versions);
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
    matches_original(&source, SHELL_TOKENS, &versions);
}
