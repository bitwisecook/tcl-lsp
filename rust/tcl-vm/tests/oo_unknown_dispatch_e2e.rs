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

//! TclOO unknown-method dispatch and filters, differential against tclsh.
//!
//! A method-lookup miss runs the object's `unknown` handlers in precedence
//! order (inherited, class and object mixins, per-object), behind the active
//! filters, with the missed name prepended to the arguments; `next` past the
//! last handler reaches `oo::object`'s built-in, which raises the standard
//! `unknown method` error. A bare `$obj` reaches the handlers with no name and
//! no filters. This is what lets retcl's `$r -sync get k` dispatch.
//!
//! `EXPECTED` is tclsh 9.1.0's output (8.6.18 prints the same); the live
//! comparison runs whenever a `tclsh9.1` / `tclsh8.6` is on `PATH` or named
//! by `TCL_LSP_TCLSH91` / `TCL_LSP_TCLSH86`.

use std::cell::RefCell;
use std::io::Write as _;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

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

fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = service
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_compiler(Box::new(service));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&capture.0.borrow())
        .trim()
        .to_owned()
}

fn tclsh_output(bin_env: &str, name: &str, src: &str) -> Option<String> {
    let candidates = std::env::var(bin_env)
        .ok()
        .into_iter()
        .chain(std::iter::once(name.to_string()));
    for bin in candidates {
        let Ok(mut child) = std::process::Command::new(bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(src.as_bytes())
            .expect("write Tcl script");
        let output = child.wait_with_output().expect("run Tcl script");
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
    }
    None
}

/// Named objects only, so no anonymous `::oo::ObjN` counter leaks into the
/// output. Each probe prints `label: code:result`.
const SCRIPT: &str = r#"proc t {label script} { set c [catch {uplevel 1 $script} r]; puts "$label: $c:$r" }
oo::class create C { method unknown {m args} { return "unknown:$m:$args" } }
C create o
t minimal {o -sync get k}
t noargs {o foo}
t direct {o unknown a b}
t bare {o}
oo::class create D { superclass C }
D create d
t inherited {d bar 1 2}
oo::class create E { superclass C; method unknown {m args} { return "E([self method]):[next $m {*}$args]" } }
E create e
t chained {e baz q}
t notfilter {catch {oo::define E method unknown {m args} {self target}}; e x}
oo::class create F { method unknown {m args} { next $m {*}$args } }
F create f
t terminal {f zap}
oo::class create G { method unknown {m args} { next } }
G create g
t terminalnoargs {g zap}
oo::class create M { method unknown {m args} { return "M:$m:[next $m {*}$args]" } }
oo::class create H { mixin M; method hello {} { return hi } }
H create h
t classmixin {h nope 1}
t classmixinhit {h hello}
oo::object create p
oo::objdefine p method unknown {m args} { return "obj:$m" }
t objmethod {p what}
oo::objdefine p mixin M
t objmixin {p what}
oo::class create Fi {
    filter logit
    method logit args { return "f([self target]|[self method]|[self filter]|$args)>[next {*}$args]" }
    method real {x} { return real$x }
    method unknown {m args} { return "unk:$m:$args" }
}
Fi create fi
t filterhit {fi real 1}
t filterunknown {fi nope 2 3}
t filterbare {fi}
oo::objdefine fi { filter g; method g args { return "g([self target])>[next {*}$args]" } }
t objfilter {fi nope 4}
t filtersinfo {list [info class filters Fi] [info object filters fi]}
oo::define Fi filter -clear
t filtercleared {fi nope 5}
oo::class create Fj { filter logit; method logit args { return "f>[next {*}$args]" } }
Fj create fj
t filternohandler {fj nope 2}
oo::class create Fd { filter a; filter a b; method a args { return a>[next {*}$args] }; method b args { return b>[next {*}$args] }; method m {} { return m } }
Fd create fd
t filterdups {list [info class filters Fd] [fd m]}
oo::class create K { method unknown {m args} { my other $m }; method other {m} { return other:$m } }
K create k
t internal {k x}
oo::class create Q { method unknown {m args} { return Q:$m }; method x {} { my nope } }
Q create q
t mymiss {q x}
oo::class create L { unexport unknown; method unknown {m args} { return L:$m } }
L create l
t redefined {list [info class methods L] [l x] [l unknown zz]}
oo::class create U { method unknown {m} { return U:$m } }
U create u
t arity {u a b}
t infocall {info object call o -sync}
t infoclasscall {info class call Fj zz}
t infocallnohandler {info class call Fj logit}
oo::class create N { method unknown {m args} { self call } }
N create n
t selfcall {n zz}
oo::class create R {
    variable async
    constructor {args} { set async 0 }
    method unknown {args} {
        if {![llength $args]} { return }
        switch -- [lindex $args 0] {
            -sync { set args [lrange $args 1 end] }
            -async { set async 1; set args [lrange $args 1 end] }
        }
        error Disconnected
    }
}
R create r -noconnect
t retclshape {r -sync get k}
t retclbare {r}
"#;

const EXPECTED: &str = r#"minimal: 0:unknown:-sync:get k
noargs: 0:unknown:foo:
direct: 0:unknown:a:b
bare: 1:wrong # args: should be "o m ?arg ...?"
inherited: 0:unknown:bar:1 2
chained: 0:E(unknown):unknown:baz:q
notfilter: 1:not inside a filtering context
terminal: 1:unknown method "zap": must be destroy or unknown
terminalnoargs: 1:wrong # args: should be "next method ?arg ...?"
classmixin: 1:unknown method "nope": must be destroy, hello or unknown
classmixinhit: 0:hi
objmethod: 0:obj:what
objmixin: 0:M:what:obj:what
filterhit: 0:f(::Fi real|logit|::Fi class logit|1)>real1
filterunknown: 0:f(::Fi unknown|logit|::Fi class logit|nope 2 3)>unk:nope:2 3
filterbare: 1:wrong # args: should be "fi m ?arg ...?"
objfilter: 0:g(::Fi unknown)>f(::Fi unknown|logit|::Fi class logit|nope 4)>unk:nope:4
filtersinfo: 0:logit g
filtercleared: 0:g(::Fi unknown)>unk:nope:5
filternohandler: 1:unknown method "nope": must be destroy or logit
filterdups: 0:{a a b} a>b>m
internal: 0:other:x
mymiss: 0:Q:nope
redefined: 0:unknown L:x L:zz
arity: 1:wrong # args: should be "u m"
infocall: 0:{unknown unknown ::C method} {unknown unknown ::oo::object {core method: "unknown"}}
infoclasscall: 0:{filter logit ::Fj method} {unknown unknown ::oo::object {core method: "unknown"}}
infocallnohandler: 0:{filter logit ::Fj method} {method logit ::Fj method}
selfcall: 0:{{unknown unknown ::N method} {unknown unknown ::oo::object {core method: "unknown"}}} 0
retclshape: 1:Disconnected
retclbare: 0:"#;

const VERSIONS: &[(TclVersion, &str, &str)] = &[
    (TclVersion::V8_6, "TCL_LSP_TCLSH86", "tclsh8.6"),
    (TclVersion::V9_1, "TCL_LSP_TCLSH91", "tclsh9.1"),
];

#[test]
fn unknown_and_filter_dispatch_match_pinned_tclsh_output() {
    for (version, _, _) in VERSIONS {
        assert_eq!(vm_output(SCRIPT, *version), EXPECTED, "{version:?}");
    }
}

#[test]
fn unknown_and_filter_dispatch_match_real_tclsh_when_available() {
    let mut ran = 0;
    for (version, env, bin) in VERSIONS {
        if let Some(actual) = tclsh_output(env, bin, SCRIPT) {
            assert_eq!(actual, EXPECTED, "{bin}");
            assert_eq!(vm_output(SCRIPT, *version), actual, "{version:?}");
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: no versioned tclsh binaries found");
    }
}
