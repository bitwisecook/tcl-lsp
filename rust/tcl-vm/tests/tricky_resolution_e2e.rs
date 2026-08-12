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

//! Tricky-feature × command-resolution pins for the bytecode VM.
//!
//! Every expectation below was pinned against real tclsh — **both** 8.6.16
//! and a locally-built 9.0.4 agree on each: `interp alias` targets resolve
//! from the GLOBAL namespace at call time (caller's frame kept), `TclOO`
//! `forward` targets resolve in the OBJECT's namespace (never the dispatch
//! site's), a `proc`-defined `tcl::mathfunc::*` function is dispatchable
//! from `expr` (TIP 232) with current-namespace-relative shadowing,
//! `namespace unknown` handlers are per-namespace / not inherited / beat
//! the global `unknown` proc, and `rename` finds its source via the full
//! resolution rule (`namespace path` included).

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string)`.
fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
    )
}

/// The result string of a script expected to succeed.
fn result(src: &str) -> String {
    let (ok, res) = run(src);
    assert!(ok, "script errored: {res}");
    res
}

// -- interp alias: global-namespace target, caller frame kept ---------------

#[test]
fn alias_target_resolves_globally_not_in_caller_namespace() {
    // tclsh: alias target `atgt` dispatches ::atgt even when invoked from
    // ::nsq which has its own atgt.
    let res = result(
        "proc ::atgt {} { return GLOBAL }\n\
         namespace eval ::nsq { proc atgt {} { return NSQ } }\n\
         interp alias {} ::al {} atgt\n\
         namespace eval ::nsq { al }",
    );
    assert_eq!(res, "GLOBAL");
}

#[test]
fn alias_keeps_the_callers_frame() {
    // tclsh: an alias to `set` writes the calling proc's LOCAL, not a global.
    let res = result(
        "interp alias {} myset {} set\n\
         proc p {} { myset x 5; list $x [info exists ::x] }\n\
         p",
    );
    assert_eq!(res, "5 0");
}

// -- TclOO forward: object-namespace target resolution -----------------------

#[test]
fn forward_target_resolves_in_object_namespace_not_caller() {
    // tclsh: `$obj fw` called from ::pkg5 (which has its own ftgt2) still
    // dispatches the GLOBAL ftgt2 — object-ns → global, never caller-ns.
    let res = result(
        "namespace eval ::pkg5 {\n\
             proc ftgt2 {args} { return PKG5 }\n\
             oo::class create FF { forward fw ftgt2 }\n\
         }\n\
         proc ::ftgt2 {args} { return GLOBALF2 }\n\
         set o [::pkg5::FF new]\n\
         namespace eval ::pkg5 [list $o fw]",
    );
    assert_eq!(res, "GLOBALF2");
}

// -- expr math functions: TIP 232 procs + namespace shadowing ----------------

#[test]
fn proc_defined_mathfunc_is_callable_from_expr() {
    let res = result(
        "proc tcl::mathfunc::square {x} { expr {$x * $x} }\n\
         expr { square(7) }",
    );
    assert_eq!(res, "49");
}

#[test]
fn namespace_local_mathfunc_shadows_global_in_expr() {
    // tclsh: expr resolves `pf(…)` as the relative name tcl::mathfunc::pf
    // from the CURRENT namespace, so ::nsa::tcl::mathfunc::pf wins inside
    // ::nsa and the global wins elsewhere.
    let res = result(
        "proc ::tcl::mathfunc::pf {x} { return 10 }\n\
         namespace eval ::nsa::tcl::mathfunc {}\n\
         proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
         list [namespace eval ::nsa { expr {pf(1)} }] [expr {pf(1)}]",
    );
    assert_eq!(res, "20 10");
}

#[test]
fn unknown_math_function_reports_the_command_miss() {
    // tclsh 8.6.16 / 9.0.4 (compiled and dynamic expr alike):
    // invalid command name "tcl::mathfunc::frobnicate"
    let (ok, res) = run("expr { frobnicate(1) }");
    assert!(!ok);
    assert!(
        res.contains("invalid command name \"tcl::mathfunc::frobnicate\""),
        "got: {res}"
    );
}

// -- namespace unknown: per-namespace, not inherited, beats ::unknown --------

#[test]
fn namespace_unknown_handler_fires_for_misses_in_that_namespace() {
    let res = result(
        "proc ::nshandler {args} { return \"H:$args\" }\n\
         namespace eval ::nsg { namespace unknown ::nshandler }\n\
         namespace eval ::nsg { nosuchcmd 1 2 }",
    );
    assert_eq!(res, "H:nosuchcmd 1 2");
}

#[test]
fn namespace_unknown_is_not_inherited_by_children() {
    // tclsh: a child of ::nsg does NOT get ::nsg's handler; with no global
    // handler and no `unknown` proc the miss is a hard error.
    let (ok, res) = run("proc ::nshandler {args} { return \"H:$args\" }\n\
         namespace eval ::nsg { namespace unknown ::nshandler }\n\
         namespace eval ::nsg::child {}\n\
         namespace eval ::nsg::child { nosuchcmd 3 }");
    assert!(!ok);
    assert!(res.contains("invalid command name"), "got: {res}");
}

#[test]
fn namespace_unknown_beats_the_global_unknown_proc() {
    // tclsh: a namespace's own handler wins; namespaces without one fall
    // back to the plain `unknown` proc.
    let res = result(
        "proc ::unknown {args} { return \"GU:$args\" }\n\
         proc ::nsh2handler {args} { return \"NSH:$args\" }\n\
         namespace eval ::nsl { namespace unknown ::nsh2handler }\n\
         list [namespace eval ::nsl { missing1 }] [namespace eval ::nsm { missing2 }]",
    );
    assert_eq!(res, "NSH:missing1 GU:missing2");
}

#[test]
fn namespace_unknown_query_reports_handler_or_default() {
    let res = result(
        "namespace eval ::nsl { namespace unknown ::h }\n\
         list [namespace eval ::nsl { namespace unknown }] [namespace unknown]",
    );
    assert_eq!(res, "::h ::unknown");
}

// -- rename: source found via the full resolution rule (namespace path) ------

#[test]
fn rename_finds_its_source_via_namespace_path() {
    // tclsh: `rename viap viap_renamed` inside ::pc (path → ::pr) moves
    // ::pr::viap; the destination roots at the CURRENT namespace (::pc).
    let res = result(
        "namespace eval ::pr { proc viap {} { return VIAP } }\n\
         namespace eval ::pc { namespace path ::pr }\n\
         namespace eval ::pc { rename viap viap_renamed }\n\
         list [llength [info commands ::pr::viap]] [llength [info commands ::pc::viap_renamed]]",
    );
    assert_eq!(res, "0 1");
}

// Issue #934: a lone colon is an ordinary name character — `proc :` is a
// real command, callable bare; the written all-colon forms name the global
// `{}` command instead (tclsh 8.6/9.0-pinned; C 8.4→9.1-invariant).
#[test]
fn colon_named_proc_dispatches_bare_but_not_via_written_runs() {
    assert_eq!(
        result("proc : args { return hello }\n: a"),
        "hello",
        "the user-reported #934 shape"
    );
    let (ok, msg) = run("proc : args { return hello }\n::: a");
    assert!(!ok, "written `:::` must not reach the `:` proc: {msg}");
    assert!(msg.contains("invalid command name"), "{msg}");
    // With the empty-named proc defined, `::` and `:::` both dispatch it.
    assert_eq!(
        result("proc {} args { return EMPTY }\nproc : args { return COLON }\n:::"),
        "EMPTY",
    );
    // Inside the namespace named `:`, a bare `:` prefers the nested proc —
    // reachable only relatively (`namespace inscope : :` in real Tcl).
    assert_eq!(
        result(
            "namespace eval : { proc : args { return NESTED } }\n\
             proc : args { return GLOBAL }\n\
             namespace eval : { : }"
        ),
        "NESTED",
    );
}

// #934 follow-up: the W314 case (proc `:` inside namespace `:`) has no
// absolute spelling but IS reachable relatively — `namespace eval : { : }`
// and `namespace inscope : :` both dispatch it, while the textual
// `namespace inscope ::: :` form resolves the all-colon spelling to the
// GLOBAL namespace and misses (tclsh 8.6.16 and 9.0.4 agree on all three;
// 9.0's `namespace code` round-trip succeeds only via a generation-time
// nsName intrep — a value-identity behaviour no written name can express,
// see docs/design/name-resolution.md §2).
#[test]
fn colon_proc_in_colon_namespace_is_relatively_reachable_only() {
    let res = result(
        "namespace eval : { proc : args { return hello } }\n\
         namespace eval : { : }",
    );
    assert_eq!(res, "hello");
    let res = result(
        "namespace eval : { proc : args { return hello } }\n\
         namespace inscope : :",
    );
    assert_eq!(res, "hello");
    let (ok, msg) = run("namespace eval : { proc : args { return hello } }\n\
         namespace inscope ::: :");
    assert!(!ok, "the textual all-colon spelling lands on global: {msg}");
    assert_eq!(msg, "invalid command name \":\"");
}
