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

/// Run `src` on a VM configured as `tclvm --tcl-version` configures one.
fn vm_output(src: &str, release: TclVersion, entry: Entry) -> String {
    let profile = tcl_registry::model::resolve_environment(release.dialect_name()).unit_profile();
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = match entry {
        Entry::File => vm.eval_source_directly(src),
        Entry::Stdin => vm.eval_source(src),
    }
    .expect("script compiles");
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
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

const VECTORS: &[Vector] = &[
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
