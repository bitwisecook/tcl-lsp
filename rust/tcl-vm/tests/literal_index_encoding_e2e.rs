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

//! **A literal index reads as the emulated release compiles it, and a word
//! read at run time as its command reads it** (#2429).
//!
//! 8.6 compiles a literal index of `string range`, `lindex` and `lrange`
//! through `TclIndexEncode`, which places an `end` offset that is positive
//! after the 32-bit wrap after the end, whatever the length. The same word
//! read at run time, through a variable, sums with `end` in an `int` and past
//! `INT_MAX` wraps before the first element. 8.4 and 8.5 wrap both; 9.0 and
//! 9.1 read both as a wide. `tclsh` compiles every body, every script `source`
//! reads and every command stdin feeds it, but evaluates the commands of the
//! script file it is given one at a time without compiling them, so a literal
//! there reads as at run time: the VM runs that file with
//! `Vm::eval_source_directly`, and stdin with `Vm::eval_source`.
//!
//! The same literal shows which scripts 8.6 compiles in a procedure frame and
//! which values its string commands leave canonical lists, and the harness
//! also pins 8.4's narrower `catch` and the error codes each release sets.
//!
//! The vectors are measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0,
//! each script run as the file `tclsh` is given and on its stdin, and are
//! compared with the release's `tclsh` again wherever one is installed.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::Vm;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// How `tclsh` is handed the script.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// The script file on its command line, whose commands it does not compile.
    File,
    /// Its stdin, each command of which it compiles.
    Stdin,
}

/// Run `src` on a VM configured as `tclvm --tcl-version` configures one, and
/// as it runs the script file or its stdin.
fn vm_output(src: &str, release: TclVersion, entry: Entry) -> String {
    let profile = tcl_registry::model::resolve_environment(release.dialect_name()).unit_profile();
    let capture = Capture::default();
    // `tclvm` runs a native-core interpreter of the release (work's entry).
    let mut vm = Vm::with_native_core(
        Box::new(capture.clone()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("native core for the selected profile");
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    match entry {
        Entry::File => {
            let completion = vm.eval_source_directly(src).expect("script compiles");
            assert!(completion.code.is_ok(), "{}", completion.result.to_str());
        }
        Entry::Stdin => vm.eval_source_commands(src, |_, message| panic!("{message}")),
    }
    let out = capture.0.borrow().clone();
    String::from_utf8(out).expect("utf-8")
}

/// Run `src` under the release's `tclsh` as `entry` hands it over, `None`
/// where no such interpreter is installed.
fn tclsh_output(src: &str, release: TclVersion, entry: Entry) -> Option<String> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let outcome = match entry {
        Entry::File => {
            let path = std::env::temp_dir().join(format!(
                "literal-index-{}-{}.tcl",
                std::process::id(),
                release.version_string()
            ));
            std::fs::write(&path, src).expect("write script");
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
            tcl_test_support::run_script(&tclsh.path, src.as_bytes())
                .expect("run tclsh")
                .stdout
        }
    };
    Some(String::from_utf8(outcome).expect("utf-8"))
}

/// One script and what each release prints for it, as a file and on stdin.
struct Vector {
    name: &'static str,
    script: &'static str,
    /// `(release, as the file, on stdin)`.
    want: &'static [(TclVersion, &'static str, &'static str)],
}

/// The issue's program and its top-level twin: `end+2147483647`.
const END_PLUS: &str = r#"proc lit {} {
    set s abcdefghijkl
    set i end+2147483647
    puts "lit range: <[string range $s 0 end+2147483647]>"
    puts "var range: <[string range $s 0 $i]>"
    puts "lit lrange: <[lrange {a b c d e f g h i j k l} 0 end+2147483647]>"
    puts "var lrange: <[lrange {a b c d e f g h i j k l} 0 $i]>"
}
lit
set s abcdefghijkl
set i end+2147483647
puts "top lit range: <[string range $s 0 end+2147483647]>"
puts "top var range: <[string range $s 0 $i]>"
set r [lrange {a b c d e f g h i j k l} 0 end+2147483647]
puts "top lit lrange: <$r>"
if 1 {
    puts "body lit range: <[string range $s 0 end+2147483647]>"
}
puts "band: <[lrange {a b c} 0 end+1073741824]> <[string range abc end-1073741825 end]>"
"#;

const WRAPPED: &str = "lit range: <>\nvar range: <>\nlit lrange: <>\nvar lrange: <>\n\
top lit range: <>\ntop var range: <>\ntop lit lrange: <>\nbody lit range: <>\n\
band: <a b c> <abc>\n";

const WIDE: &str = "lit range: <abcdefghijkl>\nvar range: <abcdefghijkl>\n\
lit lrange: <a b c d e f g h i j k l>\nvar lrange: <a b c d e f g h i j k l>\n\
top lit range: <abcdefghijkl>\ntop var range: <abcdefghijkl>\n\
top lit lrange: <a b c d e f g h i j k l>\nbody lit range: <abcdefghijkl>\n\
band: <a b c> <abc>\n";

const END_PLUS_86_FILE: &str = "lit range: <abcdefghijkl>\nvar range: <>\n\
lit lrange: <a b c d e f g h i j k l>\nvar lrange: <>\n\
top lit range: <>\ntop var range: <>\ntop lit lrange: <>\n\
body lit range: <abcdefghijkl>\nband: <a b c> <abc>\n";

const END_PLUS_86_STDIN: &str = "lit range: <abcdefghijkl>\nvar range: <>\n\
lit lrange: <a b c d e f g h i j k l>\nvar lrange: <>\n\
top lit range: <abcdefghijkl>\ntop var range: <>\n\
top lit lrange: <a b c d e f g h i j k l>\nbody lit range: <abcdefghijkl>\n\
band: <a b c> <abc>\n";

/// `end-2147483649`, an offset below `INT_MIN` that 8.4 also spells: its
/// 32-bit wrap is `end+2147483647`.
const END_MINUS: &str = r#"proc lit {} {
    set s abcdefghijkl
    set l {a b c d e f g h i j k l}
    set i end-2147483649
    puts "lit: <[string range $s end-2147483649 end]> <[lrange $l end-2147483649 end]> <[string index $s end-2147483649]> <[lindex $l end-2147483649]>"
    puts "var: <[string range $s $i end]> <[lrange $l $i end]> <[string index $s $i]> <[lindex $l $i]>"
    catch {string range $s end-2147483649 end} r
    puts "catch: <$r>"
}
lit
set s abcdefghijkl
set l {a b c d e f g h i j k l}
puts "top: <[string range $s end-2147483649 end]> <[lrange $l end-2147483649 end]>"
foreach _ x {
    puts "body: <[string range $s end-2147483649 end]> <[lrange $l end-2147483649 end]>"
}
"#;

const END_MINUS_READ: &str = "lit: <abcdefghijkl> <a b c d e f g h i j k l> <> <>\n\
var: <abcdefghijkl> <a b c d e f g h i j k l> <> <>\ncatch: <abcdefghijkl>\n\
top: <abcdefghijkl> <a b c d e f g h i j k l>\n\
body: <abcdefghijkl> <a b c d e f g h i j k l>\n";

const END_MINUS_86_FILE: &str = "lit: <> <> <> <>\n\
var: <abcdefghijkl> <a b c d e f g h i j k l> <> <>\ncatch: <>\n\
top: <abcdefghijkl> <a b c d e f g h i j k l>\nbody: <> <>\n";

const END_MINUS_86_STDIN: &str = "lit: <> <> <> <>\n\
var: <abcdefghijkl> <a b c d e f g h i j k l> <> <>\ncatch: <>\n\
top: <> <>\nbody: <> <>\n";

/// `linsert`, `lreplace` and `string replace`, which 8.6 also compiles through
/// `TclIndexEncode`: a literal `end` offset after the end appends or leaves
/// the string alone where the same word read at run time wraps before the
/// first element. `string replace $s 0 end-2147483649 X` is the exception:
/// `TclCompileStringReplaceCmd` cannot settle a replacement between an
/// absolute first index and a last one after the end, so it reads its words
/// at run time even in a proc.
const REPLACE: &str = r#"proc lit {} {
    set l {a b c d e f g h i j k l}
    set s abcdefghijkl
    set i end-2147483649
    puts "lit: <[linsert $l end-2147483649 X]> <[lreplace $l 2 end-2147483649 X]> <[string replace $s end-2147483649 end X]> <[string replace $s 0 end-2147483649 X]>"
    puts "var: <[linsert $l $i X]> <[lreplace $l 2 $i X]> <[string replace $s $i end X]> <[string replace $s 0 $i X]>"
    set r [linsert $l end-2147483649 X]
    catch {lreplace $l 2 end-2147483649 X} c
    puts "set: <$r> <$c>"
}
lit
set l {a b c d e f g h i j k l}
set s abcdefghijkl
puts "top: <[linsert $l end-2147483649 X]> <[lreplace $l 2 end-2147483649 X]> <[string replace $s end-2147483649 end X]>"
foreach _ x {
    puts "body: <[linsert $l end-2147483649 X]> <[lreplace $l 2 end-2147483649 X]> <[string replace $s end-2147483649 end X]>"
}
"#;

const REPLACE_READ: &str = "lit: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X> <abcdefghijkl>\n\
var: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X> <abcdefghijkl>\n\
set: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l>\n\
top: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X>\n\
body: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X>\n";

const REPLACE_86_FILE: &str = "lit: <a b c d e f g h i j k l X> <a b X> <abcdefghijkl> <abcdefghijkl>\n\
var: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X> <abcdefghijkl>\n\
set: <a b c d e f g h i j k l X> <a b X>\n\
top: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X>\n\
body: <a b c d e f g h i j k l X> <a b X> <abcdefghijkl>\n";

const REPLACE_86_STDIN: &str = "lit: <a b c d e f g h i j k l X> <a b X> <abcdefghijkl> <abcdefghijkl>\n\
var: <X a b c d e f g h i j k l> <a b X c d e f g h i j k l> <X> <abcdefghijkl>\n\
set: <a b c d e f g h i j k l X> <a b X>\n\
top: <a b c d e f g h i j k l X> <a b X> <abcdefghijkl>\n\
body: <a b c d e f g h i j k l X> <a b X> <abcdefghijkl>\n";

/// A script that is a canonical list, or a concatenation of them, runs as one
/// command without being compiled (`TclEvalObjEx`), wherever it runs, so 8.6
/// reads its literal index as at run time; the same script as a string it
/// compiles. Every release takes the same path, which only 8.6 shows.
const EVAL_LIST: &str = r#"set s abcdefghijkl
puts "eval list: <[eval [list string range $s 0 end-2147483649]]>"
puts "eval concat: <[eval [list string range] [list $s 0 end-2147483649]]>"
puts "eval string: <[eval "string range $s 0 end-2147483649"]>"
puts "uplevel: <[uplevel #0 [list string range $s 0 end-2147483649]]>"
puts "catch: [catch [list string range $s 0 end-2147483649] r] <$r>"
puts "if: <[if 1 [list string range $s 0 end-2147483649]]>"
puts "namespace eval: <[namespace eval :: [list string range $s 0 end-2147483649]]>"
proc p {s} {
    list <[eval [list string range $s 0 end-2147483649]]> <[eval {string range $s 0 end-2147483649}]>
}
puts "proc: [p $s]"
proc q {} {
    list <[eval [list string range abcdefghijkl 0 end-2147483649]]> \
        <[if 1 [list string range abcdefghijkl 0 end-2147483649]]>
}
puts "literal list: <[eval [list string range abcdefghijkl 0 end-2147483649]]> [q]"
"#;

const EVAL_LIST_READ: &str = "eval list: <>\neval concat: <>\neval string: <>\nuplevel: <>\n\
catch: 0 <>\nif: <>\nnamespace eval: <>\nproc: <> <>\nliteral list: <> <> <>\n";

const EVAL_LIST_86: &str = "eval list: <>\neval concat: <>\neval string: <abcdefghijkl>\n\
uplevel: <>\ncatch: 0 <>\nif: <>\nnamespace eval: <>\nproc: <> <abcdefghijkl>\n\
literal list: <> <> <>\n";

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

const ERROR_CODES_NONE: &str = "top: NONE / NONE / NONE / NONE / NONE\nproc: NONE NONE NONE NONE\n";

const ERROR_CODES_TAGGED: &str = "top: TCL WRONGARGS / TCL WRONGARGS / TCL WRONGARGS / \
TCL VALUE LIST QUOTE / TCL VALUE LIST JUNK\n\
proc: {TCL WRONGARGS} {TCL WRONGARGS} {TCL WRONGARGS} {TCL VALUE LIST BRACE}\n";

/// A command with one literal index word and one read at run time follows
/// its compile procedure's own rule: `TclCompileLreplaceCmd`,
/// `TclCompileLrangeCmd` and `TclCompileStringReplaceCmd` need both literal
/// and read both as the command runs otherwise, while
/// `TclCompileStringRangeCmd` settles a literal `first` after the end alone.
const MIXED: &str = r#"proc p {} {
    set l {a b c}; set s abcdef; set i 1
    lappend o [lreplace $l end-2147483649 $i X] [lreplace $l $i end-2147483649 X] [lreplace $l 0 end-2147483649 X]
    lappend o [string range abcdef end-2147483649 $i] [string range $s $i end-2147483649] [string range $s 0 end-2147483649]
    lappend o [lrange $l end-2147483649 $i] [lrange $l $i end-2147483649] [linsert $l end-2147483649 X]
    lappend o [string replace $s end-2147483649 $i X] [string replace $s $i end-2147483649 X] [string replace $s $i end-2147483649]
    set r1 [lreplace $l end-2147483649 $i X]; set r2 [string range abcdef end-2147483649 $i]
    lappend o $r1 $r2
}
puts "proc: [p]"
set l {a b c}; set s abcdef; set i 1
puts "top: [list [lreplace $l end-2147483649 $i X] [string range abcdef end-2147483649 $i] [lrange $l $i end-2147483649] [string replace $s $i end-2147483649]]"
"#;

const MIXED_READ: &str = "proc: {X c} {a X b c} {X a b c} ab {} {} {a b} {} {X a b c} Xcdef abcdef \
abcdef {X c} ab\ntop: {X c} ab {} abcdef\n";

const MIXED_86_FILE: &str = "proc: {X c} {a X b c} X {} {} abcdef {a b} {} {a b c X} Xcdef abcdef \
abcdef {X c} {}\ntop: {X c} ab {} abcdef\n";

const MIXED_86_STDIN: &str = "proc: {X c} {a X b c} X {} {} abcdef {a b} {} {a b c X} Xcdef abcdef \
abcdef {X c} {}\ntop: {X c} {} {} abcdef\n";

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

const STRING_READS_NONE: &str = "top: rx=- rxpat=- rxglob=- matchpat=- matchstr=- trim=- trimright=- isalpha=- \
isascii=- isint=- replnew=- replall=- replnone=- mapkey=- mapval=- maplong=- \
mappairs=- mapnone=- cat=- repeat=-\nproc: rx=- rxpat=- rxglob=- matchpat=- \
matchstr=- trim=- trimright=- isalpha=- isascii=- isint=- replnew=- replall=- \
replnone=- mapkey=- mapval=- maplong=- mappairs=- mapnone=- cat=- repeat=-\n";

const STRING_READS_86_FILE: &str = "top: rx=C rxpat=C rxglob=C matchpat=C matchstr=- trim=C trimright=C isalpha=- \
isascii=- isint=- replnew=- replall=C replnone=C mapkey=C mapval=C maplong=- \
mappairs=C mapnone=- cat=- repeat=-\nproc: rx=- rxpat=C rxglob=- matchpat=- \
matchstr=- trim=- trimright=- isalpha=C isascii=C isint=- replnew=C replall=- \
replnone=- mapkey=C mapval=C maplong=- mappairs=C mapnone=- cat=- repeat=-\n";

const STRING_READS_86_STDIN: &str = "top: rx=- rxpat=C rxglob=- matchpat=- matchstr=- trim=- trimright=- isalpha=C \
isascii=C isint=- replnew=C replall=- replnone=- mapkey=C mapval=C maplong=- \
mappairs=C mapnone=- cat=- repeat=-\nproc: rx=- rxpat=C rxglob=- matchpat=- \
matchstr=- trim=- trimright=- isalpha=C isascii=C isint=- replnew=C replall=- \
replnone=- mapkey=C mapval=C maplong=- mappairs=C mapnone=- cat=- repeat=-\n";

const VECTORS: &[Vector] = &[
    Vector {
        name: "what 8.4 refuses to compile",
        script: COMPILE_FAILURES,
        want: &[(TclVersion::V8_4, COMPILE_FAILURES_84, COMPILE_FAILURES_84)],
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
        name: "a literal index beside one read at run time",
        script: MIXED,
        want: &[
            (TclVersion::V8_4, MIXED_READ, MIXED_READ),
            (TclVersion::V8_5, MIXED_READ, MIXED_READ),
            (TclVersion::V8_6, MIXED_86_FILE, MIXED_86_STDIN),
            (TclVersion::V9_0, MIXED_READ, MIXED_READ),
            (TclVersion::V9_1, MIXED_READ, MIXED_READ),
        ],
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
    Vector {
        name: "linsert, lreplace and string replace",
        script: REPLACE,
        want: &[
            (TclVersion::V8_4, REPLACE_READ, REPLACE_READ),
            (TclVersion::V8_5, REPLACE_READ, REPLACE_READ),
            (TclVersion::V8_6, REPLACE_86_FILE, REPLACE_86_STDIN),
            (TclVersion::V9_0, REPLACE_READ, REPLACE_READ),
            (TclVersion::V9_1, REPLACE_READ, REPLACE_READ),
        ],
    },
    Vector {
        name: "a canonical-list script",
        script: EVAL_LIST,
        want: &[
            (TclVersion::V8_4, EVAL_LIST_READ, EVAL_LIST_READ),
            (TclVersion::V8_5, EVAL_LIST_READ, EVAL_LIST_READ),
            (TclVersion::V8_6, EVAL_LIST_86, EVAL_LIST_86),
            (TclVersion::V9_0, EVAL_LIST_READ, EVAL_LIST_READ),
            (TclVersion::V9_1, EVAL_LIST_READ, EVAL_LIST_READ),
        ],
    },
    Vector {
        name: "end+2147483647",
        script: END_PLUS,
        want: &[
            (TclVersion::V8_5, WRAPPED, WRAPPED),
            (TclVersion::V8_6, END_PLUS_86_FILE, END_PLUS_86_STDIN),
            (TclVersion::V9_0, WIDE, WIDE),
            (TclVersion::V9_1, WIDE, WIDE),
        ],
    },
    Vector {
        name: "end-2147483649",
        script: END_MINUS,
        want: &[
            (TclVersion::V8_4, END_MINUS_READ, END_MINUS_READ),
            (TclVersion::V8_5, END_MINUS_READ, END_MINUS_READ),
            (TclVersion::V8_6, END_MINUS_86_FILE, END_MINUS_86_STDIN),
            (TclVersion::V9_0, END_MINUS_READ, END_MINUS_READ),
            (TclVersion::V9_1, END_MINUS_READ, END_MINUS_READ),
        ],
    },
];

#[test]
fn a_literal_index_reads_as_the_release_compiles_it() {
    for vector in VECTORS {
        for &(release, file, stdin) in vector.want {
            for (entry, want) in [(Entry::File, file), (Entry::Stdin, stdin)] {
                assert_eq!(
                    vm_output(vector.script, release, entry),
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
