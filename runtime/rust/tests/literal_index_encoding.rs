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
//! The same literal shows which scripts 8.6 compiles — a substituted script a
//! command compiles with it, a `catch` a script compiles in a procedure's
//! frame where that procedure has the variable as a compiled local — and
//! which values its string commands leave canonical lists. The harness also
//! pins 8.4's narrower `catch`, the scripts 8.4 refuses to compile whole and
//! the error codes each release sets.
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

/// 8.6 converts a value its character-indexed string commands read to a
/// string (`Tcl_GetUnicodeFromObj`), so a canonical list read so is one no
/// longer and a later `eval` compiles it; an empty `string map`, `string
/// equal` and `llength` leave it a list.
const CHARS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc show {tag v} { upvar 1 out out; lappend out "$tag=<[eval $v]>" }
set out {}
set l [L]; string length $l; show length $l
set l [L]; string index $l 0; show index $l
set l [L]; string range $l 0 1; show range $l
set l [L]; string first a $l; show first $l
set l [L]; string last $l abc; show last $l
set l [L]; string reverse $l; show reverse $l
set l [L]; string map {zz y} $l; show map $l
set l [L]; string map {} $l; show emptymap $l
set l [L]; string length $l; llength $l; show relisted $l
set l [L]; string equal $l x; show equal $l
set l [L]; llength $l; show llength $l
proc p {} {
    set out {}
    set l [L]; string length $l; show length $l
    set l [L]; set v [string index $l 0]; show index $l
    set l [L]; set v [string range $l 0 1]; show range $l
    set l [L]; set v [string first a $l]; show first $l
    set l [L]; set v [string reverse $l]; show reverse $l
    set l [L]; set v [string map {a b} $l]; show map $l
    set l [L]; string equal $l x; show equal $l
    return $out
}
lappend out proc [p]
"#;

const CHARS_READ: &str = "length=<> index=<> range=<> first=<> last=<> reverse=<> map=<> \
emptymap=<> relisted=<> equal=<> llength=<> proc {length=<> index=<> range=<> first=<> \
reverse=<> map=<> equal=<>}";

const CHARS_86: &str = "length=<abcdefghijkl> index=<abcdefghijkl> range=<abcdefghijkl> \
first=<abcdefghijkl> last=<abcdefghijkl> reverse=<abcdefghijkl> map=<abcdefghijkl> \
emptymap=<> relisted=<abcdefghijkl> equal=<> llength=<> proc {length=<abcdefghijkl> \
index=<abcdefghijkl> range=<abcdefghijkl> first=<abcdefghijkl> reverse=<abcdefghijkl> \
map=<abcdefghijkl> equal=<>}";

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
    Vector {
        name: "a canonical list read by character",
        script: CHARS,
        want: &[
            (TclVersion::V8_5, CHARS_READ, CHARS_READ),
            (TclVersion::V8_6, CHARS_86, CHARS_86),
            (TclVersion::V9_0, CHARS_READ, CHARS_READ),
            (TclVersion::V9_1, CHARS_READ, CHARS_READ),
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

/// The vectors that print what they find; several read it through `expr`,
/// which a build without libtommath does not register.
#[cfg(have_tommath)]
mod printed {
    use std::cell::RefCell;
    use std::rc::Rc;

    use tcl_dialect::TclVersion;
    use tcl_host_native::NativeHost;
    use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
    use tcl_runtime::interp::Interp;
    use tcl_runtime::CompletionCode;

    use super::{Entry, Vector};

    /// A native host whose standard output the test reads.
    struct CaptureHost {
        native: NativeHost,
        stdout: Rc<RefCell<Vec<u8>>>,
    }

    impl StdIo for CaptureHost {
        fn write_stdout(&self, bytes: &[u8]) {
            self.stdout.borrow_mut().extend_from_slice(bytes);
        }

        fn write_stderr(&self, _bytes: &[u8]) {}
    }

    impl Host for CaptureHost {
        fn capabilities(&self) -> Capabilities {
            self.native.capabilities()
        }

        fn clock(&self) -> &dyn Clock {
            self.native.clock()
        }

        fn stdio(&self) -> &dyn StdIo {
            self
        }

        fn env(&self) -> &dyn Env {
            self.native.env()
        }

        fn filesystem(&self) -> Option<&dyn Filesystem> {
            self.native.filesystem()
        }

        fn process(&self) -> Option<&dyn Process> {
            self.native.process()
        }
    }

    /// What the script prints, run on the runtime as `entry` says.
    fn runtime_printed(script: &str, release: TclVersion, entry: Entry) -> String {
        let stdout = Rc::new(RefCell::new(Vec::new()));
        let mut interp = Interp::with_host(Rc::new(CaptureHost {
            native: NativeHost::new(),
            stdout: Rc::clone(&stdout),
        }));
        interp.set_runtime_version(release);
        let completion = match entry {
            Entry::File => interp.eval_sourced_completion(script.as_bytes(), b"literal-index.tcl"),
            Entry::Stdin => interp.eval_compiled_completion(script.as_bytes()),
        };
        let result = String::from_utf8(completion.result).expect("utf-8");
        assert_eq!(completion.code, CompletionCode::Ok, "{result}");
        interp.eval_completion(b"flush stdout");
        let printed = stdout.borrow().clone();
        String::from_utf8(printed).expect("utf-8")
    }

    /// What the script prints under the release's `tclsh`, `None` where no such
    /// interpreter is installed.
    fn tclsh_printed(script: &str, release: TclVersion, entry: Entry) -> Option<String> {
        let tclsh = tcl_test_support::witness_tclsh(release)?;
        let stdout = match entry {
            Entry::File => {
                let path = std::env::temp_dir().join(format!(
                    "runtime-literal-index-printed-{}-{}.tcl",
                    std::process::id(),
                    release.version_string()
                ));
                std::fs::write(&path, script).expect("write script");
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
                tcl_test_support::run_script(&tclsh.path, script.as_bytes())
                    .expect("run tclsh")
                    .stdout
            }
        };
        Some(String::from_utf8(stdout).expect("utf-8"))
    }

    /// The script words `lmap`, `dict map`/`update`/`with`, `try`, `switch` and
    /// `interp eval` evaluate at run time: a canonical list among them runs
    /// without being compiled, as `eval`'s does, a body `switch` splits from a
    /// string compiles, and `[list …]` of literal words is a list too, which 8.6
    /// pushes as its list object. On stdin 8.6 compiles the top-level `try $b`,
    /// which compiles `$b` and leaves it a string for every later command.
    const BODIES: &str = r#"set b [list string range abcdefghijkl 0 end-2147483649]
set d {a 1}
puts "lmap: <[lmap x {1} $b]> <[lmap x {1} [list string range abcdefghijkl 0 end-2147483649]]>"
puts "dict: <[dict map {k v} $d $b]> <[dict update d a v $b]> <[dict with d $b]>"
puts "try: <[try $b]> <[try {error x} on error {} $b]> <[try $b on ok {r} {set r}]>"
puts "switch: <[switch a a $b]> <[switch -glob a b {} a* $b]> <[switch a [list a $b]]> <[switch a "a {$b}"]>"
puts "interp eval: <[interp eval {} $b]>"
proc p {b} {
    set d {a 1}
    set c [list string range abcdefghijkl 0 end-2147483649]
    list <[lmap x {1} $b]> <[lmap x {1} $c]> <[dict map {k v} $d $b]> \
        <[dict update d a v $b]> <[dict with d $b]> <[switch a a $b]> \
        <[try {error x} on error {} $b]> <[if 1 $c]>
}
puts "proc: [p $b]"
"#;

    const BODIES_READ: &str = "lmap: <{}> <{}>\ndict: <a {}> <> <>\ntry: <> <> <>\n\
    switch: <> <> <> <>\ninterp eval: <>\nproc: <{}> <{}> {<a {}>} <> <> <> <> <>\n";

    const BODIES_86_FILE: &str = "lmap: <{}> <{}>\ndict: <a {}> <> <>\ntry: <> <> <>\n\
    switch: <> <> <> <abcdefghijkl>\ninterp eval: <>\nproc: <{}> <{}> {<a {}>} <> <> <> <> <>\n";

    const BODIES_86_STDIN: &str = "lmap: <{}> <{}>\ndict: <a {}> <> <>\n\
    try: <abcdefghijkl> <abcdefghijkl> <abcdefghijkl>\n\
    switch: <abcdefghijkl> <abcdefghijkl> <abcdefghijkl> <abcdefghijkl>\n\
    interp eval: <abcdefghijkl>\n\
    proc: <abcdefghijkl> <{}> {<a abcdefghijkl>} <abcdefghijkl> <abcdefghijkl> <abcdefghijkl> <abcdefghijkl> <>\n";

    /// The script word a release compiles with its command whatever it holds
    /// (`TclCompileCmdWord`'s `INST_EVAL_STK`): `catch` with procedure-local
    /// variables or none, `try` whose handlers it compiles, and a `for` start
    /// beside literal clauses. A canonical list there compiles, and stays
    /// compiled for a later `eval` of the same value; a `trap {}` handler, a
    /// qualified variable or a script compiled outside a procedure leave it to
    /// the command, which evaluates it directly.
    const COMPILED: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc p1 {b} { catch $b r; set r }
proc p2 {b} { try $b }
proc p3 {b} { try $b on ok {r} {set r} }
proc p4 {b} { catch $b; eval $b }
proc p5 {b} { for $b {0} {} {}; eval $b }
proc p6 {b} { try $b trap {} {} {} }
proc p7 {b} { catch $b ::r; set ::r }
proc p8 {b} { lmap x {1} $b; eval $b }
proc p9 {} { catch [list string range abcdefghijkl 0 end-2147483649] r; set r }
foreach p {p1 p2 p3 p4 p5 p6 p7 p8} { lappend out <[$p [L]]> }
puts "proc: $out <[p9]>"
set b [L]; set t1 <[eval {try $b}]>
set b [L]; eval {catch $b r}; set t2 <$r>
set b [L]; catch $b; set t3 <[eval $b]>
puts "script: $t1 $t2 $t3"
"#;

    const COMPILED_READ: &str = "proc: <> <> <> <> <> <> <> <> <>\nscript: <> <> <>\n";

    const COMPILED_86_FILE: &str = "proc: <abcdefghijkl> <abcdefghijkl> <abcdefghijkl> \
    <abcdefghijkl> <abcdefghijkl> <> <> <> <abcdefghijkl>\nscript: <abcdefghijkl> <> <>\n";

    const COMPILED_86_STDIN: &str = "proc: <abcdefghijkl> <abcdefghijkl> <abcdefghijkl> \
    <abcdefghijkl> <abcdefghijkl> <> <> <> <abcdefghijkl>\n\
    script: <abcdefghijkl> <> <abcdefghijkl>\n";

    /// A script `eval`, `uplevel` or `apply` compiles in a procedure frame
    /// creates no locals of its own: `catch` compiles with a result or options
    /// variable only where the frame's procedure already has it as a compiled
    /// local (`TclFindCompiledLocal` reads the frame's `localCachePtr`), and a
    /// `try` with a handler, which needs locals of its own, never compiles there.
    const FRAME_LOCALS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc local {b} { set r x; eval {catch $b r}; set r }
proc later {b} { set r x; eval {catch $b r}; eval $b }
proc notlocal {b} { eval {catch $b zz}; eval $b }
proc novar {b} { eval {catch $b}; eval $b }
proc both {b} { set r 1; set o 1; eval {catch $b r o}; set r }
proc onelocal {b} { set r 1; eval {catch $b r o}; set r }
proc element {b} { set r(x) 1; eval {catch $b r(x)}; set r(x) }
proc linked {b} { global g; eval {catch $b g}; set g }
proc nested {b} { set r 1; eval {eval {catch $b r}}; set r }
proc level0 {b} { set r 1; uplevel 0 {catch $b r}; set r }
proc caller {b} { set r 1; callee $b; set r }
proc callee {b} { uplevel 1 {catch $b r} }
proc text {b} { set r 1; set s {catch $b r}; eval $s; set r }
proc handler {b} { set r 1; eval {try $b on ok {r} {set r}} }
proc lambda {b} { apply {{b} {set r 1; eval {catch $b r}; set r}} $b }
foreach p {local later notlocal novar both onelocal element linked nested level0 caller text handler lambda} {
    lappend out "$p=<[$p [L]]>"
}
puts $out
set r 1; eval {catch [L] r}; puts "global=<$r>"
"#;

    const FRAME_LOCALS_READ: &str = "local=<> later=<> notlocal=<> novar=<> both=<> onelocal=<> \
    element=<> linked=<> nested=<> level0=<> caller=<> text=<> handler=<> lambda=<>\nglobal=<>\n";

    const FRAME_LOCALS_86: &str = "local=<abcdefghijkl> later=<abcdefghijkl> notlocal=<> \
    novar=<abcdefghijkl> both=<abcdefghijkl> onelocal=<> element=<> linked=<abcdefghijkl> \
    nested=<abcdefghijkl> level0=<abcdefghijkl> caller=<abcdefghijkl> text=<abcdefghijkl> \
    handler=<> lambda=<abcdefghijkl>\nglobal=<>\n";

    /// 8.6 converts a value its character-indexed string commands read to a
    /// string, so a canonical list read so is one no longer and a later `eval`
    /// compiles it; `string equal`, `string match`, an empty `string map` and the
    /// list commands leave it a list. `append` onto an unset variable stores the
    /// value appended itself, and appending an empty string leaves a value alone.
    const SHIMMER: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc show {tag v} { upvar 1 out out; lappend out "$tag=<[eval $v]>" }
set out {}
set l [L]; string length $l; show length $l
set l [L]; string index $l 0; show index $l
set l [L]; string range $l 0 1; show range $l
set l [L]; string first a $l; show first $l
set l [L]; string last $l abc; show last $l
set l [L]; string replace $l 0 0 s; show replace $l
set l [L]; string map {zz y} $l; show map $l
set l [L]; string map {} $l; show emptymap $l
set l [L]; regsub a $l b; show regsub $l
set l [L]; string length $l; llength $l; show relisted $l
set l [L]; string equal $l x; show equal $l
set l [L]; string match x* $l; show match $l
set l [L]; string toupper $l; show toupper $l
set l [L]; llength $l; show llength $l
set l [L]; unset -nocomplain v; append v $l; show append $v
set l [L]; unset -nocomplain v; append v $l ""; show appendempty $v
set l [L]; set v $l; append v ""; show appendnothing $v
set l [L]; set v ""; append v $l; show appendstring $v
puts "top: $out"
proc p {} {
    set out {}
    set l [L]; string length $l; show length $l
    set l [L]; set v [string index $l 0]; show index $l
    set l [L]; set v [string range $l 0 1]; show range $l
    set l [L]; set v [string first a $l]; show first $l
    set l [L]; set v [string last $l abc]; show last $l
    set l [L]; set v [string map {a b} $l]; show map $l
    set l [L]; set v [string is alpha $l]; show isalpha $l
    set l [L]; regsub a $l b; show regsub $l
    set l [L]; string equal $l x; show equal $l
    set l [L]; unset -nocomplain v; append v $l; show append $v
    set l [L]; append w $l ""; show appendempty $w
    set l [L]; set v ""; append v $l; show appendstring $v
    return $out
}
puts "proc: [p]"
"#;

    const SHIMMER_READ: &str = "top: length=<> index=<> range=<> first=<> last=<> replace=<> \
    map=<> emptymap=<> regsub=<> relisted=<> equal=<> match=<> toupper=<> llength=<> append=<> \
    appendempty=<> appendnothing=<> appendstring=<>\nproc: length=<> index=<> range=<> first=<> \
    last=<> map=<> isalpha=<> regsub=<> equal=<> append=<> appendempty=<> appendstring=<>\n";

    const SHIMMER_86: &str = "top: length=<abcdefghijkl> index=<abcdefghijkl> \
    range=<abcdefghijkl> first=<abcdefghijkl> last=<abcdefghijkl> replace=<abcdefghijkl> \
    map=<abcdefghijkl> emptymap=<> regsub=<abcdefghijkl> relisted=<abcdefghijkl> equal=<> \
    match=<> toupper=<> llength=<> append=<> appendempty=<> appendnothing=<> \
    appendstring=<abcdefghijkl>\nproc: length=<abcdefghijkl> index=<abcdefghijkl> \
    range=<abcdefghijkl> first=<abcdefghijkl> last=<abcdefghijkl> map=<abcdefghijkl> \
    isalpha=<abcdefghijkl> regsub=<abcdefghijkl> equal=<> append=<> appendempty=<> \
    appendstring=<abcdefghijkl>\n";

    /// A script held in a variable is compiled by `eval` at run time as a unit
    /// of its own, in the procedure's frame, even where its text is known: a
    /// `catch` there compiles its script only where each variable is already a
    /// compiled local of the procedure (`$zz` read anywhere in the body makes it
    /// one), so folding the text into the body must not make it one.
    const EVAL_TEXT: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc t1 {b} { set s {catch $b zz}; eval $s; eval $b }
proc t2 {b} { set r 1; set s {catch $b r}; eval $s; set r }
proc t3 {b} { set s {catch $b zz}; eval $s; return $zz }
proc t4 {b} { set s {catch $b}; eval $s; eval $b }
proc t5 {b} { set s {catch $b zz yy}; eval $s; list $zz }
foreach p {t1 t2 t3 t4 t5} { lappend out "$p=<[$p [L]]>" }
puts "proc: $out"
set s {catch $b zz}; set b [L]; eval $s; puts "top: <[eval $b]>"
"#;

    const EVAL_TEXT_READ: &str = "proc: t1=<> t2=<> t3=<> t4=<> t5=<{}>\ntop: <>\n";

    const EVAL_TEXT_86: &str =
        "proc: t1=<> t2=<abcdefghijkl> t3=<abcdefghijkl> t4=<abcdefghijkl> t5=<{}>\ntop: <>\n";

    /// Which variables a procedure's compiled locals include, as 8.6 shows it:
    /// a script `eval` compiles in the frame compiles `catch $b r` only where
    /// `r` is already one (`TclFindCompiledLocal`), and a compiled `catch` reads
    /// `$b`'s literal index apart. C makes a word a compiled local wherever a
    /// compile procedure compiles it, even where the command never runs: a
    /// one-word `set` or `append` in any position, an `info exists` element, the
    /// arrays of `array set` and of a pattern-less `array unset`, loop and
    /// handler variables, the variables of a `catch` (an element names its
    /// array's, and stops the compile) and of the scripts it compiles with it,
    /// `variable`'s tails and `namespace upvar`'s locals; not those of a command
    /// it leaves to run (`gets`, a `regexp` with match variables, `array names`),
    /// a qualified name, an element `variable` cannot link, nor a script behind
    /// a constant-false `if` or `while`.
    const COMPILED_LOCALS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
set shapes {
    {set r} {set r(x)} {set x [set r]} {list [set r] 1} {set x "a[set r]"} {expr {[set r]}}
    {append r} {info exists r(x)} {array set r {}} {array unset r} {array unset r x}
    {lassign {a} r} {lset r 1} {foreach r {1} {}} {foreach {x r} {} {}} {lmap r {} {}}
    {dict update d a r {}} {dict with r {}} {dict map {r v} {} {}} {catch {} r}
    {catch {} r(x)} {catch {} ::r} {catch {foreach r {} {}}} {try {} on ok r {}}
    {variable r} {variable r(x)} {namespace upvar :: x r} {gets stdin r}
    {regexp a a r} {array names r} {if 0 {set r}} {while 0 {set r 1}}
}
set i 0
foreach shape $shapes {
    incr i
    proc p$i {b} "if {\$b eq {}} { $shape }; eval {catch \$b r}; eval \$b"
    append out [expr {[p$i [L]] eq {} ? {-} : {C}}]
}
puts $out
"#;

    const COMPILED_LOCALS_READ: &str = "--------------------------------\n";

    const COMPILED_LOCALS_86: &str = "CCCCCCCCCC-CCCCCCCCCC-CCC-C-----\n";

    /// A canonical list `eval`, `uplevel`, `if` or `catch` runs directly hands
    /// its command the very values the list holds (`Tcl_EvalObjv` over its
    /// elements), so a canonical list among them stays one: `eval [list catch $b
    /// r]` gives `catch` the list `$b` is, which it runs directly in turn. A
    /// script 8.6 compiles (the canonical list a compiled `catch` evaluates with
    /// `INST_EVAL_STK`, a string `eval`) makes its words afresh.
    const DIRECT_WORDS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc show {tag v} { upvar 1 out out; lappend out "$tag=<$v>" }
proc p1 {b} { set r 1; eval [list catch $b r]; set r }
proc p2 {b} { eval [list set r $b]; eval $r }
proc p3 {b} { set r 1; uplevel 0 [list catch $b r]; set r }
proc p4 {b} { set r 1; catch [list catch $b r]; set r }
proc p5 {b} { set r 1; if 1 [list catch $b r]; set r }
proc p6 {b} { set r 1; eval [list catch] [list $b r]; set r }
proc p7 {b} { set r 1; eval "catch {$b} r"; set r }
set out {}
foreach p {p1 p2 p3 p4 p5 p6 p7} { show $p [$p [L]] }
set r 1; eval [list catch [L] r]; show top $r
set c [list set q [L]]; eval $c; show topset [eval $q]
puts $out
"#;

    const DIRECT_WORDS_READ: &str = "p1=<> p2=<> p3=<> p4=<> p5=<> p6=<> p7=<> top=<> topset=<>\n";

    const DIRECT_WORDS_86: &str =
        "p1=<> p2=<> p3=<> p4=<abcdefghijkl> p5=<> p6=<> p7=<abcdefghijkl> top=<> topset=<>\n";

    /// What each string command and `regexp` does to the value it reads, where
    /// it is compiled and where it runs as a command, which shows as 8.6 reading
    /// the canonical list it was apart afterwards. A command reads its words by
    /// character (`Tcl_GetUnicodeFromObj`), a pattern as a regexp, or not at
    /// all, and some return a word itself: `regexp` reads both, compiled only
    /// where its literal pattern is no glob `TclReToGlob` can make; `string
    /// match` reads both where the string is untyped or a string, compiled only
    /// where either is already a string; `string is` reads its value only
    /// compiled; the compiled `string trim` returns a value it trims nothing
    /// from, the command never; `string replace` reads its replacement only
    /// compiled, and settles a literal no-op there; `string map` reads its keys
    /// and values, a lone pair's value only where its key could match; and an
    /// empty map, `string cat` of one word and `string repeat` once return the
    /// word.
    const STRING_READS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
proc show {tag v} { upvar 1 out out; lappend out "$tag=[expr {[eval $v] eq {} ? {-} : {C}}]" }
set out {}
set l [L]; regexp abc $l; show rx $l
set l [L]; regexp $l abc; show rxpat $l
set l [L]; set v [regexp -nocase {^a.c$} $l]; show rxglob $l
set l [L]; string match $l t1; show matchpat $l
set l [L]; string match t2* $l; show matchstr $l
set l [L]; set v [string trim $l]; show trim $v
set l [L]; set v [string trimright $l x]; show trimright $v
set l [L]; string is alpha $l; show isalpha $l
set l [L]; set v [string is ascii $l]; show isascii $l
set l [L]; set v [string is integer $l]; show isint $l
set l [L]; string replace abc 1 1 $l; show replnew $l
set l [L]; set v [string replace abc 0 2 $l]; show replall $v
set l [L]; set v [string replace $l 5 2 x]; show replnone $v
set l [L]; string map [list $l x] abc; show mapkey $l
set l [L]; string map [list x $l] abc; show mapval $l
set l [L]; string map [list xyzw $l] abc; show maplong $l
set l [L]; string map [list x $l y z] abc; show mappairs $l
set l [L]; set v [string map {} $l]; show mapnone $v
set l [L]; set v [string cat $l]; show cat $v
set l [L]; set v [string repeat $l 1]; show repeat $v
puts "top: $out"
proc p {} {
    set out {}
    set l [L]; regexp abc $l; show rx $l
    set l [L]; regexp $l abc; show rxpat $l
    set l [L]; set v [regexp -nocase {^a.c$} $l]; show rxglob $l
    set l [L]; string match $l t3; show matchpat $l
    set l [L]; string match t4* $l; show matchstr $l
    set l [L]; set v [string trim $l]; show trim $v
    set l [L]; set v [string trimright $l x]; show trimright $v
    set l [L]; string is alpha $l; show isalpha $l
    set l [L]; set v [string is ascii $l]; show isascii $l
    set l [L]; set v [string is integer $l]; show isint $l
    set l [L]; string replace abc 1 1 $l; show replnew $l
    set l [L]; set v [string replace abc 0 2 $l]; show replall $v
    set l [L]; set v [string replace $l 5 2 x]; show replnone $v
    set l [L]; string map [list $l x] abc; show mapkey $l
    set l [L]; string map [list x $l] abc; show mapval $l
    set l [L]; string map [list xyzw $l] abc; show maplong $l
    set l [L]; string map [list x $l y z] abc; show mappairs $l
    set l [L]; set v [string map {} $l]; show mapnone $v
    set l [L]; set v [string cat $l]; show cat $v
    set l [L]; set v [string repeat $l 1]; show repeat $v
    return $out
}
puts "proc: [p]"
"#;

    const STRING_READS_NONE: &str =
        "top: rx=- rxpat=- rxglob=- matchpat=- matchstr=- trim=- trimright=- isalpha=- \
    isascii=- isint=- replnew=- replall=- replnone=- mapkey=- mapval=- maplong=- \
    mappairs=- mapnone=- cat=- repeat=-\nproc: rx=- rxpat=- rxglob=- matchpat=- \
    matchstr=- trim=- trimright=- isalpha=- isascii=- isint=- replnew=- replall=- \
    replnone=- mapkey=- mapval=- maplong=- mappairs=- mapnone=- cat=- repeat=-\n";

    const STRING_READS_86_FILE: &str =
        "top: rx=C rxpat=C rxglob=C matchpat=C matchstr=- trim=C trimright=C isalpha=- \
    isascii=- isint=- replnew=- replall=C replnone=C mapkey=C mapval=C maplong=- \
    mappairs=C mapnone=- cat=- repeat=-\nproc: rx=- rxpat=C rxglob=- matchpat=- \
    matchstr=- trim=- trimright=- isalpha=C isascii=C isint=- replnew=C replall=- \
    replnone=- mapkey=C mapval=C maplong=- mappairs=C mapnone=- cat=- repeat=-\n";

    const STRING_READS_86_STDIN: &str =
        "top: rx=- rxpat=C rxglob=- matchpat=- matchstr=- trim=- trimright=- isalpha=C \
    isascii=C isint=- replnew=C replall=- replnone=- mapkey=C mapval=C maplong=- \
    mappairs=C mapnone=- cat=- repeat=-\nproc: rx=- rxpat=C rxglob=- matchpat=- \
    matchstr=- trim=- trimright=- isalpha=C isascii=C isint=- replnew=C replall=- \
    replnone=- mapkey=C mapval=C maplong=- mappairs=C mapnone=- cat=- repeat=-\n";

    /// More of the shapes whose variables 8.6 makes compiled locals, and those it
    /// does not: the variables of a constant `if` or `while`'s untaken scripts,
    /// of `dict`'s loops and `dict update`, of `upvar` and `namespace upvar`
    /// (scalars), `global` and `variable` (tails), of `try`'s handlers and of a
    /// `switch` it compiles (a pattern list, or words after `--`), of `subst`'s
    /// substitutions and of a word that substitutes an element; and not those of
    /// `regexp`, `regsub`, a `catch` that stops at a qualified name before its
    /// script, or a script `eval`, `uplevel`, `namespace eval`, `apply` or `time`
    /// runs.
    const MORE_LOCALS: &str = r#"proc L {} { list string range abcdefghijkl 0 [set e end-2147483649] }
set shapes {
    {incr r} {lappend r x} {unset r} {unset -nocomplain r} {global r} {global ::ns::r} {variable ::ns::r} {upvar 1 x r} {upvar x r}
    {namespace upvar :: x r q y} {array exists r} {array get r} {array size r} {dict set r a 1} {dict unset r a} {dict incr r a}
    {dict append r a x} {dict lappend r a x} {dict get $r a} {dict exists $r a} {dict for {r v} {} {}} {dict filter {} script {r v} {}}
    {dict with d {set r 1}} {dict update d a x {set r 1}} {lmap x {} {set r 1}} {foreach {a b} {} {set r 1}} {while 1 {set r 1; break}}
    {for {set r 0} {0} {} {}} {for {} {$r} {} {}} {switch a a {set r 1}} {switch -- a a {set r 1}} {switch a {a {set r 1}}}
    {expr {$r}} {if {$r} {}} {puts $r} {set x $r(1)} {set x ${r}} {list "$r"} {proc q {} {set r 1}} {after 0 {set r 1}}
    {namespace eval :: {set r 1}} {eval {set r 1}} {uplevel 1 {set r 1}} {time {set r 1}} {subst {$r}} {string length $r}
    {regsub a b c r} {scan a %s r} {binary scan a a r} {lassign {} a r} {lset r 0 1} {append r x y} {set r x} {set ::r x}
    {catch {} r o} {catch {} a r} {catch {set r 1} x} {try {set r 1} on ok {} {}} {try {} trap {} r {}} {if 0 {} else {set r 1}}
    {if 1 {} else {set r 1}} {if 0 {} elseif 1 {set r 1}} {if {$x} {set r 1}} {while {0} {set r 1}} {while {$x} {set r 1}}
    {info exists r} {array unset r} {array set r {a b}} {array names r *} {variable r} {variable q 1 r} {global q r}
    {catch {} r(x) r} {set r(x) 1} {incr r(x)} {lappend ::r x} {foreach r(x) {} {}} {foreach x {} {set r 1}}
    {if 0 then {set r 1}} {if 1 then {} else {set r 1}} {if 0 {} elseif 0 {set r 1} else {}} {if 0 {} elseif 0 {} else {set r 1}}
    {if 0 {} elseif {$x} {set r 1}} {if {1} {set r 1}} {if no {set r 1}} {if yes {} {set r 1}} {if 0 {} {set r 1}}
    {while 1 {set r 1}} {while {1} {set r 1}} {while false {set r 1}} {for {} 0 {set r 1} {}} {for {} {0} {} {set r 1}}
    {foreach {a b} {} {c r} {} {}} {foreach a {} r(x) {} {}} {foreach ::a {} r {} {}} {foreach {} {} r {} {}}
    {lmap {a r} {} {}} {dict map {a r} {} {}} {dict for {r} {} {}} {dict update r a b {}} {dict update d a r(x) {}}
    {dict with r {}} {dict with ::r {}} {dict set r(x) a 1} {dict incr ::q a} {dict get r a}
    {variable a 1 r} {variable a 1 r(x) 2 q} {global a r(x)} {global ::a::b r} {upvar 1 x r(x)} {upvar #0 x ::ns::r}
    {catch {} q r} {catch {} r(x) q} {catch {} ::q r} {catch {set r 1} ::q}
    {switch -exact -- a a {set r 1}} {switch -glob -- a a - b {set r 1}} {switch -nocase -- a a {set r 1}} {switch -exact -nocase -- a a {set r 1}}
    {switch -- a {a {set r 1}}} {switch a {a -}} {switch -regexp -- a a {set r 1}} {switch -- $a a {set r 1}}
    {try {} on error {r} {}} {try {} on error {a r} {}} {try {} on error {a b r} {set r 1}} {try {} trap {A} {r} {}}
    {try {} o error {r} {}} {try {set r 1} finally {}} {try {} finally {set r 1}} {try {} on error {} - on ok {r} {}}
    {expr {[set r]}} {expr $q + [set r]} {expr {$r(1)}} {expr {${r}}} {set q [expr {$::r}]}
    {subst -nocommands {$r}} {subst -novariables {$r}} {subst -novariables {[set r 1]}} {subst {[set r 1]}}
    {incr ::r} {info exists ::r} {lappend r(1) x} {lset r(1) 0 1} {lassign {} ::r q} {append $q r}
    {string length [set r]} {list [list [set r]]} {puts "[set r]"} {set q {$r}} {set q "$r"} {set q \$r}
    {after 0 [set r]} {namespace eval :: [set r]} {array set ::r {}} {array exists r} {array exists r(x)} {array unset r(x)}
    {info exists r($q)} {set r($q) 1} {set ${r} 1} {set "r" 1} {incr r 2} {unset -nocomplain -- r} {unset r(x)}
    {regexp -- a b} {regexp a b r} {regsub -all a b c} {regsub -all a b c r} {lmap r {} {set r}}
    {apply {{} {set r 1}}} {proc q {r} {}} {interp eval {} {set r 1}} {uplevel #0 {set r}} {time {set r} 1}
}
set i 0
foreach shape $shapes {
    incr i
    proc p$i {b} "if {\$b eq {}} { $shape }; eval {catch \$b r}; eval \$b"
    append out [expr {[p$i [L]] eq {} ? {-} : {C}}]
}
puts $out
"#;

    const MORE_LOCALS_READ: &str = "--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------\n";

    const MORE_LOCALS_86: &str = "CCCCCCCCCCC--CCCCCCCC-CCCCCCC-CCCCCCCC------CC---CCCC-CCCC-C-CC-CCCC-CCCCCC-CC---CCC--CCC-CCCC--CC-CCC-C--C--CC-CC--CC--C-CCCC-C-CCCCCCC-C-CC--CC--CCC-C-CC-CCCCCCCCCC----C-----\n";

    /// Tcl 8.4's `catch` takes the result variable alone; an options variable is
    /// its `wrong # args`, from the command and from the compiled `catch` alike.
    const CATCH_ARITY: &str = r#"proc stmt {} { catch {set x 1} r o; list $r $o }
proc value {} { set y [catch {set x 1} r o]; list $y $r }
proc subst {b} { catch $b r o; list $r }
proc short {} { list [catch {set x 1} r] $r }
foreach p {stmt value {subst {set x 1}} short} { lappend out [list [catch $p m] $m] }
puts "proc: $out"
puts "top: [list [catch {catch {set x 1} r o} m] $m] [list [catch {catch} m] $m]"
set c catch
puts "dynamic: [list [catch {$c {set x 1} r o} m] $m]"
"#;

    const CATCH_ARITY_84: &str = "proc: {1 {wrong # args: should be \"catch command ?varName?\"}} \
    {1 {wrong # args: should be \"catch command ?varName?\"}} \
    {1 {wrong # args: should be \"catch command ?varName?\"}} {0 {0 1}}\n\
    top: 1 {wrong # args: should be \"catch command ?varName?\"} \
    1 {wrong # args: should be \"catch command ?varName?\"}\n\
    dynamic: 1 {wrong # args: should be \"catch command ?varName?\"}\n";

    const CATCH_ARITY_85: &str = "proc: {0 {1 {-code 0 -level 0}}} {0 {0 1}} {0 1} {0 {0 1}}\n\
    top: 0 0 1 {wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"}\n\
    dynamic: 0 0\n";

    /// `Tcl_WrongNumArgs` and the list parser tag their errors `TCL WRONGARGS`
    /// and `TCL VALUE LIST …` from 8.6; 8.4 and 8.5 set no error code, so
    /// `$errorCode` is `NONE` for every command's usage error and malformed list.
    const ERROR_CODES: &str = r#"proc code {script} { catch {uplevel 1 $script}; return $::errorCode }
proc p {a} {}
proc q {} { list [code {llength}] [code {p}] [code {set}] [code [list llength "\{a"]] }
puts "top: [code {llength}] / [code {p 1 2}] / [code {string length}] / [code [list llength "\"a"]] / [code [list lindex "\{a\}b" 0]]"
puts "proc: [q]"
"#;

    const ERROR_CODES_NONE: &str =
        "top: NONE / NONE / NONE / NONE / NONE\nproc: NONE NONE NONE NONE\n";

    const ERROR_CODES_TAGGED: &str = "top: TCL WRONGARGS / TCL WRONGARGS / TCL WRONGARGS / \
    TCL VALUE LIST QUOTE / TCL VALUE LIST JUNK\n\
    proc: {TCL WRONGARGS} {TCL WRONGARGS} {TCL WRONGARGS} {TCL VALUE LIST BRACE}\n";

    /// 8.4 compiles a script whole before any of it runs: a procedure body when
    /// it is first called, a script a command evaluates, each chunk of stdin. A
    /// compile procedure's error (`catch {} a b`, `expr {1 +}`, `if 1`, `incr`)
    /// or a parse error anywhere fails the compile, so nothing in the script
    /// runs, and the error carries a `while compiling` frame per nested script,
    /// the context line a compile procedure adds and `(compiling body of proc
    /// …)`. A `catch` whose script fails to compile leaves it to run, catching
    /// the failure; an untaken constant `if` or `while` compiles no body; `eval`
    /// and `uplevel` run their scripts directly.
    const COMPILE_FAILURES: &str = r#"set ::errorInfo {}
proc show {name} {
    set c [catch {$name} m]
    set info $::errorInfo
    set cut [string last "\n    invoked from within" $info]
    puts "$name $c <$m>\n<[string range $info 0 [expr {$cut - 1}]]>"
}
proc p1 {} { puts a
  if 1 { puts b; catch {} a b } }
proc p2 {} { while 1 { catch {} a b; break } }
proc p3 {} { for {} 1 {} { puts x
   catch {} a b; break } }
proc p4 {} { foreach x {1} { catch {} a b } }
proc p5 {} { catch { catch {} a b } }
proc p6 {} { set y [if 1 {catch {} a b}] }
proc p7 {} { if {1 +} {} }
proc p8 {} { set x [expr {1 + }] }
proc p9 {} { incr }
proc p10 {} { if 1 {} else }
proc p11 {} { break x }
proc p12 {} { foreach x }
proc p13 {} { if 1 {} elseif }
proc p14 {} { set x "[catch {} a b]" }
proc p15 {} { puts a; set x {a}b }
proc p16 {} {puts a
set x [list a
}
proc p17 {} { while 0 { catch {} a b } }
proc p18 {} { if 0 { catch {} a b } }
proc p19 {} { set s {catch {} a b}; eval $s}
proc p20 {} { foreach x {1} { if {$x} then { expr {1 * * 2} } } }
foreach p {p1 p2 p3 p4 p5 p6 p7 p8 p9 p10 p11 p12 p13 p14 p15 p16 p17 p18 p19 p20} { show $p }
set c [catch {if 1 {puts b; catch {} a b}} m]; puts "top $c <$m>\n<$::errorInfo>"
set c [catch {eval {puts e; catch {} a b}} m]; puts "eval $c <$m>\n<$::errorInfo>"
set c [catch {uplevel #0 {puts u; catch {} a b}} m]; puts "uplevel $c <$m>\n<$::errorInfo>"
"#;

    const COMPILE_FAILURES_84: &str = r#"p1 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b "
    ("if" then script line 1)
    while compiling
"if 1 { puts b; catch {} a b } "
    (compiling body of proc "p1", line 2)>
p2 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b"
    ("while" body line 1)
    while compiling
"while 1 { catch {} a b; break } "
    (compiling body of proc "p2", line 1)>
p3 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b"
    ("for" body line 2)
    while compiling
"for {} 1 {} { puts x
   catch {} a b; break } "
    (compiling body of proc "p3", line 1)>
p4 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b "
    ("foreach" body line 1)
    while compiling
"foreach x {1} { catch {} a b } "
    (compiling body of proc "p4", line 1)>
p5 0 <1>
<>
p6 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b"
    ("if" then script line 1)
    while compiling
"if 1 {catch {} a b}"
    while compiling
"set y [if 1 {catch {} a b}] "
    (compiling body of proc "p6", line 1)>
p7 1 <syntax error in expression "1 +": premature end of expression>
<syntax error in expression "1 +": premature end of expression
    ("if" test expression)
    while compiling
"if {1 +} {} "
    (compiling body of proc "p7", line 1)>
p8 1 <syntax error in expression "1 + ": premature end of expression>
<syntax error in expression "1 + ": premature end of expression
    while compiling
"expr {1 + }"
    while compiling
"set x [expr {1 + }] "
    (compiling body of proc "p8", line 1)>
p9 1 <wrong # args: should be "incr varName ?increment?">
<wrong # args: should be "incr varName ?increment?"
    while compiling
"incr "
    (compiling body of proc "p9", line 1)>
p10 1 <wrong # args: no script following "else" argument>
<wrong # args: no script following "else" argument
    while compiling
"if 1 {} else "
    (compiling body of proc "p10", line 1)>
p11 1 <wrong # args: should be "break">
<wrong # args: should be "break"
    while compiling
"break x "
    (compiling body of proc "p11", line 1)>
p12 1 <wrong # args: should be "foreach varList list ?varList list ...? command">
<wrong # args: should be "foreach varList list ?varList list ...? command"
    while compiling
"foreach x "
    (compiling body of proc "p12", line 1)>
p13 1 <wrong # args: no expression after "elseif" argument>
<wrong # args: no expression after "elseif" argument
    while compiling
"if 1 {} elseif "
    (compiling body of proc "p13", line 1)>
p14 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b"
    while compiling
"set x "[catch {} a b]" "
    (compiling body of proc "p14", line 1)>
p15 1 <extra characters after close-brace>
<extra characters after close-brace
    while compiling
"set x {a}b "
    (compiling body of proc "p15", line 1)>
p16 1 <missing close-bracket>
<missing close-bracket
    while compiling
"set x [list a
"
    (compiling body of proc "p16", line 2)>
p17 0 <>
<missing close-bracket
    while compiling
"set x [list a
"
    (compiling body of proc "p16", line 2)>
p18 0 <>
<missing close-bracket
    while compiling
"set x [list a
"
    (compiling body of proc "p16", line 2)>
p19 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while executing
"catch {} a b"
    ("eval" body line 1)
    invoked from within
"eval $s"
    (procedure "p19" line 1)>
p20 1 <syntax error in expression "1 * * 2": unexpected operator *>
<syntax error in expression "1 * * 2": unexpected operator *
    while compiling
"expr {1 * * 2} "
    ("if" then script line 1)
    while compiling
"if {$x} then { expr {1 * * 2} } "
    ("foreach" body line 1)
    while compiling
"foreach x {1} { if {$x} then { expr {1 * * 2} } } "
    (compiling body of proc "p20", line 1)>
top 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {} a b"
    ("if" then script line 1)
    while compiling
"if 1 {puts b; catch {} a b}">
e
eval 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while executing
"catch {} a b"
    ("eval" body line 1)
    invoked from within
"eval {puts e; catch {} a b}">
u
uplevel 1 <wrong # args: should be "catch command ?varName?">
<wrong # args: should be "catch command ?varName?"
    while executing
"catch {} a b"
    ("uplevel" body line 1)
    invoked from within
"uplevel #0 {puts u; catch {} a b}">
"#;

    /// The error codes a malformed list leaves, from every command that parses
    /// one and from a procedure's usage error: `NONE` up to 8.5, `TCL VALUE LIST
    /// BRACE|QUOTE|JUNK` and `TCL WRONGARGS` from 8.6, though `eval` raises a
    /// script's parse error without one.
    const LIST_ERROR_CODES: &str = r#"proc code {script} { catch {uplevel 1 $script}; return $::errorCode }
proc p {} {
    set bad "\{a"
    list [code [list llength $bad]] [code [list foreach x $bad {}]] [code {llength}]
}
set bad "\{a"
set out {}
foreach s [list [list llength $bad] [list lindex "\{a\}b" 0] [list lsort $bad] [list foreach x $bad {}] [list lrange $bad 0 1] [list lsearch $bad a] [list join $bad] [list lappend bad x] [list lset bad 0 x] [list lreplace $bad 0 0] [list linsert $bad 0 x] [list concat $bad] [list eval $bad] [list switch a $bad] [list llength "\"a"] [list array set q $bad] [list lindex "\"a\"b" 0]] {
    lappend out [code $s]
}
puts "top: [join $out { / }]"
puts "proc: [p]"
"#;

    const LIST_ERROR_CODES_NONE: &str = r#"top: NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE / NONE
proc: NONE NONE NONE
"#;

    const LIST_ERROR_CODES_TAGGED: &str = r#"top: TCL VALUE LIST BRACE / TCL VALUE LIST JUNK / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / TCL VALUE LIST BRACE / NONE / TCL VALUE LIST BRACE / TCL VALUE LIST QUOTE / TCL VALUE LIST BRACE / TCL VALUE LIST JUNK
proc: {TCL VALUE LIST BRACE} {TCL VALUE LIST BRACE} {TCL WRONGARGS}
"#;

    /// Where 8.4 raises `catch`'s usage error: while compiling a script it
    /// compiles whole (a `catch` body, a procedure body, nothing of which runs),
    /// or while executing one it evaluates directly (`eval`) and a command it
    /// reaches through a substituted word.
    const CATCH_TRACE: &str = r#"set c [catch {catch {set x 1} r o} m]; puts "top $c <$m> <$::errorCode>\n<$::errorInfo>"
proc p {} { puts before; catch {set x 1} r o; puts after }
set c [catch {p} m]; puts "proc $c <$m> <$::errorCode>\n<$::errorInfo>"
set c [catch {eval {puts e; catch {set x 1} r o}} m]; puts "eval $c <$m> <$::errorCode>\n<$::errorInfo>"
set cmd catch
set c [catch {$cmd {set x 1} r o} m]; puts "dyn $c <$m> <$::errorCode>\n<$::errorInfo>"
proc q {} { set c catch; $c {set x 1} r o }
set c [catch {q} m]; puts "qdyn $c <$m> <$::errorCode>\n<$::errorInfo>"
set c [catch {catch} m]; puts "none $c <$m> <$::errorCode>\n<$::errorInfo>"
"#;

    const CATCH_TRACE_84: &str = r#"top 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {set x 1} r o">
proc 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch {set x 1} r o"
    (compiling body of proc "p", line 1)
    invoked from within
"p">
e
eval 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while executing
"catch {set x 1} r o"
    ("eval" body line 1)
    invoked from within
"eval {puts e; catch {set x 1} r o}">
dyn 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while executing
"$cmd {set x 1} r o">
qdyn 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while executing
"$c {set x 1} r o "
    (procedure "q" line 1)
    invoked from within
"q">
none 1 <wrong # args: should be "catch command ?varName?"> <NONE>
<wrong # args: should be "catch command ?varName?"
    while compiling
"catch">
"#;

    /// A malformed index from each command that reads one, at top level and
    /// in a proc: 8.4 words it `must be integer or end?-integer?` and reads no
    /// `end+N`, 8.5 on `must be integer?[+-]integer? or end?[+-]integer?`, and
    /// the error code is `TCL VALUE INDEX` from 8.6, `NONE` before it.
    const BAD_INDEX: &str = r#"proc code {script} { set c [catch {uplevel 1 $script} m]; return "$c <$m> <$::errorCode>" }
set l {a b c}; set s abc
foreach cmd [list {lindex $l x} {lindex $l end+1} {lrange $l x 1} {lrange $l 0 end+x} {linsert $l x a} {lreplace $l x x} {lset l x a} {string index $s x} {string range $s 0 x} {string replace $s x x} {string first a $s x} {string last a $s x} {string wordend $s x} {string toupper $s x} {lsearch -start x $l a} {lsort -index x {{a b}}} {lindex $l end-x} {lindex $l 1x} {string index $s end+1} {lindex $l --1} {lindex $l 0x} {lindex $l 1.0}] {
    puts "$cmd => [code $cmd]"
}
proc p {} { set l {a b c}; set s abc; list [code {lindex $l x}] [code {string range $s 0 y}] [catch {lindex $l z} m] $m [catch {string index $s q} m] $m }
puts "proc: [p]"
"#;

    const BAD_INDEX_84: &str = r#"lindex $l x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lindex $l end+1 => 1 <bad index "end+1": must be integer or end?-integer?> <NONE>
lrange $l x 1 => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lrange $l 0 end+x => 1 <bad index "end+x": must be integer or end?-integer?> <NONE>
linsert $l x a => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lreplace $l x x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lset l x a => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string index $s x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string range $s 0 x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string replace $s x x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string first a $s x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string last a $s x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string wordend $s x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
string toupper $s x => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lsearch -start x $l a => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lsort -index x {{a b}} => 1 <bad index "x": must be integer or end?-integer?> <NONE>
lindex $l end-x => 1 <bad index "end-x": must be integer or end?-integer?> <NONE>
lindex $l 1x => 1 <bad index "1x": must be integer or end?-integer?> <NONE>
string index $s end+1 => 1 <bad index "end+1": must be integer or end?-integer?> <NONE>
lindex $l --1 => 1 <bad index "--1": must be integer or end?-integer?> <NONE>
lindex $l 0x => 1 <bad index "0x": must be integer or end?-integer?> <NONE>
lindex $l 1.0 => 1 <bad index "1.0": must be integer or end?-integer?> <NONE>
proc: {1 <bad index "x": must be integer or end?-integer?> <NONE>} {1 <bad index "y": must be integer or end?-integer?> <NONE>} 1 {bad index "z": must be integer or end?-integer?} 1 {bad index "q": must be integer or end?-integer?}
"#;

    const BAD_INDEX_85: &str = r#"lindex $l x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lindex $l end+1 => 0 <> <NONE>
lrange $l x 1 => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lrange $l 0 end+x => 1 <bad index "end+x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
linsert $l x a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lreplace $l x x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lset l x a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string index $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string range $s 0 x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string replace $s x x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string first a $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string last a $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string wordend $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string toupper $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lsearch -start x $l a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lsort -index x {{a b}} => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lindex $l end-x => 1 <bad index "end-x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lindex $l 1x => 1 <bad index "1x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
string index $s end+1 => 0 <> <NONE>
lindex $l --1 => 1 <bad index "--1": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lindex $l 0x => 1 <bad index "0x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
lindex $l 1.0 => 1 <bad index "1.0": must be integer?[+-]integer? or end?[+-]integer?> <NONE>
proc: {1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <NONE>} {1 <bad index "y": must be integer?[+-]integer? or end?[+-]integer?> <NONE>} 1 {bad index "z": must be integer?[+-]integer? or end?[+-]integer?} 1 {bad index "q": must be integer?[+-]integer? or end?[+-]integer?}
"#;

    const BAD_INDEX_86: &str = r#"lindex $l x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lindex $l end+1 => 0 <> <TCL VALUE INDEX>
lrange $l x 1 => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lrange $l 0 end+x => 1 <bad index "end+x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
linsert $l x a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lreplace $l x x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lset l x a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string index $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string range $s 0 x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string replace $s x x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string first a $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string last a $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string wordend $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string toupper $s x => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lsearch -start x $l a => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lsort -index x {{a b}} => 1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lindex $l end-x => 1 <bad index "end-x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lindex $l 1x => 1 <bad index "1x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
string index $s end+1 => 0 <> <TCL VALUE INDEX>
lindex $l --1 => 1 <bad index "--1": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lindex $l 0x => 1 <bad index "0x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
lindex $l 1.0 => 1 <bad index "1.0": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>
proc: {1 <bad index "x": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>} {1 <bad index "y": must be integer?[+-]integer? or end?[+-]integer?> <TCL VALUE INDEX>} 1 {bad index "z": must be integer?[+-]integer? or end?[+-]integer?} 1 {bad index "q": must be integer?[+-]integer? or end?[+-]integer?}
"#;

    /// Scripts that print what each release does, as the file and on stdin.
    const PRINTED: &[Vector] = &[
        Vector {
            name: "catch's arity",
            script: CATCH_ARITY,
            want: &[
                (TclVersion::V8_4, CATCH_ARITY_84, CATCH_ARITY_84),
                (TclVersion::V8_5, CATCH_ARITY_85, CATCH_ARITY_85),
                (TclVersion::V8_6, CATCH_ARITY_85, CATCH_ARITY_85),
                (TclVersion::V9_0, CATCH_ARITY_85, CATCH_ARITY_85),
                (TclVersion::V9_1, CATCH_ARITY_85, CATCH_ARITY_85),
            ],
        },
        Vector {
            name: "where 8.4 raises catch's usage error",
            script: CATCH_TRACE,
            want: &[(TclVersion::V8_4, CATCH_TRACE_84, CATCH_TRACE_84)],
        },
        Vector {
            name: "what 8.4 refuses to compile",
            script: COMPILE_FAILURES,
            want: &[(TclVersion::V8_4, COMPILE_FAILURES_84, COMPILE_FAILURES_84)],
        },
        Vector {
            name: "usage and list error codes",
            script: ERROR_CODES,
            want: &[
                (TclVersion::V8_4, ERROR_CODES_NONE, ERROR_CODES_NONE),
                (TclVersion::V8_5, ERROR_CODES_NONE, ERROR_CODES_NONE),
                (TclVersion::V8_6, ERROR_CODES_TAGGED, ERROR_CODES_TAGGED),
                (TclVersion::V9_0, ERROR_CODES_TAGGED, ERROR_CODES_TAGGED),
                (TclVersion::V9_1, ERROR_CODES_TAGGED, ERROR_CODES_TAGGED),
            ],
        },
        Vector {
            name: "a malformed index",
            script: BAD_INDEX,
            want: &[
                (TclVersion::V8_4, BAD_INDEX_84, BAD_INDEX_84),
                (TclVersion::V8_5, BAD_INDEX_85, BAD_INDEX_85),
                (TclVersion::V8_6, BAD_INDEX_86, BAD_INDEX_86),
                (TclVersion::V9_0, BAD_INDEX_86, BAD_INDEX_86),
                (TclVersion::V9_1, BAD_INDEX_86, BAD_INDEX_86),
            ],
        },
        Vector {
            name: "a malformed list's error code",
            script: LIST_ERROR_CODES,
            want: &[
                (
                    TclVersion::V8_4,
                    LIST_ERROR_CODES_NONE,
                    LIST_ERROR_CODES_NONE,
                ),
                (
                    TclVersion::V8_5,
                    LIST_ERROR_CODES_NONE,
                    LIST_ERROR_CODES_NONE,
                ),
                (
                    TclVersion::V8_6,
                    LIST_ERROR_CODES_TAGGED,
                    LIST_ERROR_CODES_TAGGED,
                ),
                (
                    TclVersion::V9_0,
                    LIST_ERROR_CODES_TAGGED,
                    LIST_ERROR_CODES_TAGGED,
                ),
                (
                    TclVersion::V9_1,
                    LIST_ERROR_CODES_TAGGED,
                    LIST_ERROR_CODES_TAGGED,
                ),
            ],
        },
        Vector {
            name: "a procedure's compiled locals",
            script: COMPILED_LOCALS,
            want: &[
                (TclVersion::V8_4, COMPILED_LOCALS_READ, COMPILED_LOCALS_READ),
                (TclVersion::V8_5, COMPILED_LOCALS_READ, COMPILED_LOCALS_READ),
                (TclVersion::V8_6, COMPILED_LOCALS_86, COMPILED_LOCALS_86),
                (TclVersion::V9_0, COMPILED_LOCALS_READ, COMPILED_LOCALS_READ),
                (TclVersion::V9_1, COMPILED_LOCALS_READ, COMPILED_LOCALS_READ),
            ],
        },
        Vector {
            name: "more compiled locals",
            script: MORE_LOCALS,
            want: &[
                (TclVersion::V8_5, MORE_LOCALS_READ, MORE_LOCALS_READ),
                (TclVersion::V8_6, MORE_LOCALS_86, MORE_LOCALS_86),
                (TclVersion::V9_0, MORE_LOCALS_READ, MORE_LOCALS_READ),
                (TclVersion::V9_1, MORE_LOCALS_READ, MORE_LOCALS_READ),
            ],
        },
        Vector {
            name: "the words of a script run directly",
            script: DIRECT_WORDS,
            want: &[
                (TclVersion::V8_4, DIRECT_WORDS_READ, DIRECT_WORDS_READ),
                (TclVersion::V8_5, DIRECT_WORDS_READ, DIRECT_WORDS_READ),
                (TclVersion::V8_6, DIRECT_WORDS_86, DIRECT_WORDS_86),
                (TclVersion::V9_0, DIRECT_WORDS_READ, DIRECT_WORDS_READ),
                (TclVersion::V9_1, DIRECT_WORDS_READ, DIRECT_WORDS_READ),
            ],
        },
        Vector {
            name: "what string commands read",
            script: STRING_READS,
            want: &[
                (
                    TclVersion::V8_6,
                    STRING_READS_86_FILE,
                    STRING_READS_86_STDIN,
                ),
                (TclVersion::V9_0, STRING_READS_NONE, STRING_READS_NONE),
                (TclVersion::V9_1, STRING_READS_NONE, STRING_READS_NONE),
            ],
        },
        Vector {
            name: "a script eval compiles from a variable",
            script: EVAL_TEXT,
            want: &[
                (TclVersion::V8_5, EVAL_TEXT_READ, EVAL_TEXT_READ),
                (TclVersion::V8_6, EVAL_TEXT_86, EVAL_TEXT_86),
                (TclVersion::V9_0, EVAL_TEXT_READ, EVAL_TEXT_READ),
                (TclVersion::V9_1, EVAL_TEXT_READ, EVAL_TEXT_READ),
            ],
        },
        Vector {
            name: "a script compiled in a procedure frame",
            script: FRAME_LOCALS,
            want: &[
                (TclVersion::V8_6, FRAME_LOCALS_86, FRAME_LOCALS_86),
                (TclVersion::V9_0, FRAME_LOCALS_READ, FRAME_LOCALS_READ),
                (TclVersion::V9_1, FRAME_LOCALS_READ, FRAME_LOCALS_READ),
            ],
        },
        Vector {
            name: "a canonical list read as a string",
            script: SHIMMER,
            want: &[
                (TclVersion::V8_4, SHIMMER_READ, SHIMMER_READ),
                (TclVersion::V8_5, SHIMMER_READ, SHIMMER_READ),
                (TclVersion::V8_6, SHIMMER_86, SHIMMER_86),
                (TclVersion::V9_0, SHIMMER_READ, SHIMMER_READ),
                (TclVersion::V9_1, SHIMMER_READ, SHIMMER_READ),
            ],
        },
        Vector {
            name: "bodies a command evaluates",
            script: BODIES,
            want: &[
                (TclVersion::V8_6, BODIES_86_FILE, BODIES_86_STDIN),
                (TclVersion::V9_0, BODIES_READ, BODIES_READ),
                (TclVersion::V9_1, BODIES_READ, BODIES_READ),
            ],
        },
        Vector {
            name: "script words compiled with their command",
            script: COMPILED,
            want: &[
                (TclVersion::V8_6, COMPILED_86_FILE, COMPILED_86_STDIN),
                (TclVersion::V9_0, COMPILED_READ, COMPILED_READ),
                (TclVersion::V9_1, COMPILED_READ, COMPILED_READ),
            ],
        },
    ];

    #[test]
    fn a_script_compiles_and_a_value_shimmers_as_the_release_does() {
        for vector in PRINTED {
            for &(release, file, stdin) in vector.want {
                for (entry, want) in [(Entry::File, file), (Entry::Stdin, stdin)] {
                    assert_eq!(
                        runtime_printed(vector.script, release, entry),
                        want,
                        "{} on {release:?} from {entry:?}",
                        vector.name
                    );
                }
            }
        }
    }

    #[test]
    fn the_printed_vectors_match_each_tclsh() {
        for vector in PRINTED {
            for &(release, file, stdin) in vector.want {
                for (entry, want) in [(Entry::File, file), (Entry::Stdin, stdin)] {
                    if let Some(out) = tclsh_printed(vector.script, release, entry) {
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
}
