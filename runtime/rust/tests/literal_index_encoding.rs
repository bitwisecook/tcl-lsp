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

//! A literal index reads as the emulated release compiles it, and a word
//! read at run time as its command reads it (#2473).
//!
//! 8.6 compiles a literal index of `lindex`, `lrange`, `linsert`, `lreplace`,
//! `string range` and `string replace` through `TclIndexEncode`, which places
//! an `end` offset past the 32-bit wrap after the end; read as the command
//! runs, the same word wraps before the first element. `tclsh` compiles every
//! body, `eval` and `uplevel` script, `apply` lambda and command its stdin
//! feeds it, but evaluates the commands of the script file it is given, and a
//! canonical-list script, without compiling them. The runtime evaluates the
//! file with `Interp::eval_sourced_completion` and stdin with
//! `Interp::eval_compiled_completion`.
//!
//! The vectors are measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0,
//! and compared with the release's `tclsh` again wherever one is installed.

use tcl_dialect::TclVersion;
use tcl_runtime::interp::Interp;
use tcl_runtime::CompletionCode;

/// How `tclsh` is handed the script.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// The script file on its command line, whose commands it does not compile.
    File,
    /// Its stdin, each command of which it compiles.
    Stdin,
}

/// The value the script leaves in `out`, run on the runtime as `entry` says.
fn runtime_output(script: &str, release: TclVersion, entry: Entry) -> String {
    let mut interp = Interp::new();
    interp.set_runtime_version(release);
    let completion = match entry {
        Entry::File => interp.eval_sourced_completion(script.as_bytes(), b"literal-index.tcl"),
        Entry::Stdin => interp.eval_compiled_completion(script.as_bytes()),
    };
    let result = String::from_utf8(completion.result).expect("utf-8");
    assert_eq!(completion.code, CompletionCode::Ok, "{result}");
    result
}

/// The value the script leaves in `out` under the release's `tclsh`, `None`
/// where no such interpreter is installed.
fn tclsh_output(script: &str, release: TclVersion, entry: Entry) -> Option<String> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let source = format!("{script}\nputs -nonewline $out\n");
    let stdout = match entry {
        Entry::File => {
            let path = std::env::temp_dir().join(format!(
                "runtime-literal-index-{}-{}.tcl",
                std::process::id(),
                release.version_string()
            ));
            std::fs::write(&path, &source).expect("write script");
            let output = std::process::Command::new(&tclsh.path)
                .arg(&path)
                .env("LANG", "C.UTF-8")
                .env("LC_ALL", "C.UTF-8")
                .output()
                .expect("run tclsh");
            let _ = std::fs::remove_file(&path);
            output.stdout
        }
        Entry::Stdin => {
            tcl_test_support::run_script(&tclsh.path, source.as_bytes())
                .expect("run tclsh")
                .stdout
        }
    };
    Some(String::from_utf8(stdout).expect("utf-8"))
}

/// One script and the `out` each release leaves, as the file and on stdin.
struct Vector {
    name: &'static str,
    script: &'static str,
    /// `(release, as the file, on stdin)`.
    want: &'static [(TclVersion, &'static str, &'static str)],
}

/// `end-2147483649`, whose 32-bit wrap is `end+2147483647`, in each command
/// 8.6 encodes, beside `string index`, `lset` and `string first`, which it
/// reads as they run, and through a variable: in a proc, an `eval` and an
/// `uplevel` script, at the top level, in a command substitution there, in a
/// loop body and as a canonical-list script.
const LITERALS: &str = r#"proc probe {} {
    set s abcdefghijkl
    set l {a b c d e f g h i j k l}
    set i end-2147483649
    set x $l
    catch {lset x end-2147483649 Z} x
    list <[string index $s end-2147483649]> <[string range $s end-2147483649 end]> \
        <[lindex $l end-2147483649]> <[lindex $l end-2147483649 0]> \
        <[lrange $l 0 end-2147483649]> <[lrange $l end-2147483649 $i]> \
        <[linsert $l end-2147483649 X]> <[lreplace $l 2 end-2147483649 X]> \
        <[lreplace $l end-2147483649 $i X]> <[string replace $s 0 end-2147483649]> \
        <[string replace $s 0 end-2147483649 X]> <[string replace $s end-2147483649 end X]> \
        <[string first a $s end-2147483649]> <$x> <[string range $s $i end]>
}
set out [list proc [probe]]
lappend out eval [eval [info body probe]]
proc up {} { uplevel 1 [info body probe] }
lappend out uplevel [up]
set s abcdefghijkl
set l {a b c d e f g h i j k l}
lappend out top <[string range $s end-2147483649 end]> <[lrange $l 0 end-2147483649]> \
    <[linsert $l end-2147483649 X]> <[lindex $l end-2147483649]>
lappend out subst [list <[lrange $l 0 end-2147483649]>]
foreach _ x { lappend out body <[lrange $l 0 end-2147483649]> <[string replace $s 0 end-2147483649]> }
lappend out list <[eval [list lrange $l 0 end-2147483649]]> \
    <[eval [list lrange] [list $l 0 end-2147483649]]> \
    <[eval lrange [list $l] 0 end-2147483649]> \
    <[namespace eval :: [list lrange $l 0 end-2147483649]]>
"#;

/// The body every release but 8.6 reads alike, compiled or not.
macro_rules! read_body {
    ($lset:literal) => {
        concat!(
            "{<> <abcdefghijkl> <> <> <> <> {<X a b c d e f g h i j k l>} ",
            "{<a b X c d e f g h i j k l>} {<X a b c d e f g h i j k l>} <abcdefghijkl> ",
            "<abcdefghijkl> <X> <0> {<",
            $lset,
            ">} <abcdefghijkl>}"
        )
    };
}

macro_rules! read {
    ($lset:literal) => {
        concat!(
            "proc ",
            read_body!($lset),
            " eval ",
            read_body!($lset),
            " uplevel ",
            read_body!($lset),
            " top <abcdefghijkl> <> {<X a b c d e f g h i j k l>} <> subst <> ",
            "body <> <abcdefghijkl> list <> <> <> <>"
        )
    };
}

const LITERALS_8X: &str = read!("list index out of range");

const LITERALS_9X: &str = read!("index \"end-2147483649\" out of range");

macro_rules! compiled_body {
    () => {
        concat!(
            "{<> <> <> <> {<a b c d e f g h i j k l>} <> {<a b c d e f g h i j k l X>} ",
            "{<a b X>} {<X a b c d e f g h i j k l>} <> <abcdefghijkl> <abcdefghijkl> <0> ",
            "{<list index out of range>} <abcdefghijkl>}"
        )
    };
}

macro_rules! compiled {
    ($top:literal) => {
        concat!(
            "proc ",
            compiled_body!(),
            " eval ",
            compiled_body!(),
            " uplevel ",
            compiled_body!(),
            $top,
            " body {<a b c d e f g h i j k l>} <> list <> <> {<a b c d e f g h i j k l>} <>"
        )
    };
}

const LITERALS_86_FILE: &str =
    compiled!(" top <abcdefghijkl> <> {<X a b c d e f g h i j k l>} <> subst <>");

const LITERALS_86_STDIN: &str = compiled!(
    " top <> {<a b c d e f g h i j k l>} {<a b c d e f g h i j k l X>} <> \
     subst {{<a b c d e f g h i j k l>}}"
);

/// An `apply` lambda is a compiled body.
const LAMBDA: &str = r#"set out [apply {{} {
    set l {a b c}
    list [lrange $l 0 end-2147483649] [linsert $l end-2147483649 X]
}}]
"#;

/// 8.6 compiles a command through its compile procedure only where the words
/// that select it are literal: the command word, and an ensemble's subcommand
/// word. A command or subcommand reached through a variable, or a command
/// with a `{*}` the parser cannot expand, is a generic invocation, which reads
/// every index word as the command runs; a literal `{*}` of literal elements
/// the parser expands into literal words.
const SELECTED: &str = r#"proc p {} {
    set c lrange; set sub range; set st string; set l {a b c}; set s abcdef
    list <[$c $l 0 end-2147483649]> <[lrange $l 0 end-2147483649]> \
        <[string $sub $s 0 end-2147483649]> <[$st range $s 0 end-2147483649]> \
        <[string range $s 0 end-2147483649]> <[string ran $s 0 end-2147483649]> \
        <[{*}$c $l 0 end-2147483649]> <[lrange {*}{{a b c} 0} end-2147483649]> \
        <[{*}{string range} $s 0 end-2147483649]> <[lrange $l {*}"0 end-2147483649"]>
}
set out [p]
set c lrange; set sub range
lappend out <[$c {a b c} 0 end-2147483649]> <[string $sub abcdef 0 end-2147483649]> \
    <[lrange {a b c} 0 end-2147483649]>
"#;

const SELECTED_READ: &str = "<> <> <> <> <> <> <> <> <> <> <> <> <>";

const SELECTED_86_FILE: &str = "<> {<a b c>} <> <> <abcdef> <abcdef> <> {<a b c>} <abcdef> \
{<a b c>} <> <> <>";

const SELECTED_86_STDIN: &str = "<> {<a b c>} <> <> <abcdef> <abcdef> <> {<a b c>} <abcdef> \
{<a b c>} <> <> {<a b c>}";

const VECTORS: &[Vector] = &[
    Vector {
        name: "a literal index",
        script: LITERALS,
        want: &[
            (TclVersion::V8_4, LITERALS_8X, LITERALS_8X),
            (TclVersion::V8_5, LITERALS_8X, LITERALS_8X),
            (TclVersion::V8_6, LITERALS_86_FILE, LITERALS_86_STDIN),
            (TclVersion::V9_0, LITERALS_9X, LITERALS_9X),
            (TclVersion::V9_1, LITERALS_9X, LITERALS_9X),
        ],
    },
    Vector {
        name: "an apply lambda",
        script: LAMBDA,
        want: &[
            (TclVersion::V8_5, "{} {X a b c}", "{} {X a b c}"),
            (TclVersion::V8_6, "{a b c} {a b c X}", "{a b c} {a b c X}"),
            (TclVersion::V9_0, "{} {X a b c}", "{} {X a b c}"),
            (TclVersion::V9_1, "{} {X a b c}", "{} {X a b c}"),
        ],
    },
    Vector {
        name: "a command selected by literal words",
        script: SELECTED,
        want: &[
            (TclVersion::V8_5, SELECTED_READ, SELECTED_READ),
            (TclVersion::V8_6, SELECTED_86_FILE, SELECTED_86_STDIN),
            (TclVersion::V9_0, SELECTED_READ, SELECTED_READ),
            (TclVersion::V9_1, SELECTED_READ, SELECTED_READ),
        ],
    },
];

#[test]
fn a_literal_index_reads_as_the_release_compiles_it() {
    for vector in VECTORS {
        for &(release, file, stdin) in vector.want {
            for (entry, want) in [(Entry::File, file), (Entry::Stdin, stdin)] {
                assert_eq!(
                    runtime_output(vector.script, release, entry),
                    want,
                    "{} on {release:?} from {entry:?}",
                    vector.name
                );
            }
        }
    }
}

#[test]
fn the_vectors_match_each_tclsh() {
    for vector in VECTORS {
        for &(release, file, stdin) in vector.want {
            for (entry, want) in [(Entry::File, file), (Entry::Stdin, stdin)] {
                if let Some(out) = tclsh_output(vector.script, release, entry) {
                    assert_eq!(
                        out, want,
                        "tclsh {release:?}: {} from {entry:?}",
                        vector.name
                    );
                }
            }
        }
    }
}
