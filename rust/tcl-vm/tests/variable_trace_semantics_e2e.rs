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

//! Variable-trace semantics on the read-modify-write commands (issue #1633
//! rows 1, 3 and 4).
//!
//! Two C facts drive every vector here:
//!
//! * `TclPtrSetVarIdx` returns the variable read back **after** the write
//!   traces have run (`tclVar.c` 9.0.4:2050-2065), not the value the store
//!   was handed — so a callback that rewrites, unsets, or arrays the
//!   variable changes what `set`/`append`/`lappend`/`incr` evaluate to.
//! * `incr`, and the `lappend` paths that reach `TclPtrGetVarIdx`, fire the
//!   variable's `read` trace before the store. `incr` always does
//!   (`TclPtrIncrObjVarIdx` :2262-2272); `lappend` does only through
//!   `Tcl_LappendObjCmd` (:2895, :2944) and `INST_LAPPEND_LIST*`
//!   (`tclExecute.c:3391`) — the single-value in-proc opcodes
//!   `INST_LAPPEND_{SCALAR,ARRAY,STK,ARRAY_STK}` omit `TCL_TRACE_READS`
//!   (`tclExecute.c:3110-3121`) and fire `write` only. `append` never fires
//!   `read`.
//!
//! Every vector's stdout is compared against the bytecode VM **and** — when
//! installed — real `tclsh8.6` / `tclsh9.0`; the two releases produce
//! identical bytes for all of it (measured on 8.6.16 and 9.0.4).

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::{lower_to_ir, lower_to_ir_traced};
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

    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir_traced(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

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

/// Run `src` in the VM; the script's `puts` output is returned.
fn vm_output(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
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
            .expect("write");
        let out = child.wait_with_output().expect("run");
        if out.status.success() {
            return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
    }
    None
}

struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    // Row 1: `TclPtrSetVarIdx`'s tail returns the variable's value *after* the
    // write traces, and the empty string when the variable no longer holds a
    // scalar — the callback unset it, or turned it into an array.
    Vector {
        name: "a write trace that rewrites or removes the variable decides what the store returns",
        script: "proc mangle {n1 n2 op} { set ::x mangled }\n\
                 trace add variable x write mangle\n\
                 puts <[set x orig]>\n\
                 proc vanish {n1 n2 op} { unset ::y }\n\
                 trace add variable y write vanish\n\
                 puts <[set y orig]>|[info exists y]\n\
                 proc mangle2 {n1 n2 op} { set ::z mangled }\n\
                 trace add variable z write mangle2\n\
                 puts <[incr z]>\n\
                 set w 5\n\
                 proc vanish2 {n1 n2 op} { unset ::w }\n\
                 trace add variable w write vanish2\n\
                 puts <[incr w 3]>|\n\
                 proc toarray {n1 n2 op} { unset ::c; set ::c(e) 1 }\n\
                 set c 1\n\
                 trace add variable c write toarray\n\
                 puts <[set c orig]>|[array exists c]\n",
        want: "<mangled>\n<>|0\n<mangled>\n<>|\n<>|1",
    },
    // Row 1 again, one line per store arm the VM carries: scalar and element,
    // stack-named and slot-named, `set`/`append`/`lappend`/`incr`, at the top
    // level and inside a proc.
    Vector {
        name: "every store path returns the value read back after the write traces",
        script: "proc mang {n1 n2 op} { upvar 1 $n1 v; set v mangled }\n\
                 proc mange {n1 n2 op} { upvar 1 $n1 arr; set arr($n2) mangled }\n\
                 set a orig\n\
                 trace add variable a write mang\n\
                 puts \"append-top: <[append a x]>|$a\"\n\
                 set b orig\n\
                 trace add variable b write mang\n\
                 puts \"lappend-top: <[lappend b x]>|$b\"\n\
                 array set c {k orig}\n\
                 trace add variable c(k) write mange\n\
                 puts \"set-elem: <[set c(k) new]>|$c(k)\"\n\
                 array set d {k orig}\n\
                 trace add variable d(k) write mange\n\
                 puts \"append-elem: <[append d(k) x]>|$d(k)\"\n\
                 array set e {k orig}\n\
                 trace add variable e(k) write mange\n\
                 puts \"lappend-elem: <[lappend e(k) x]>|$e(k)\"\n\
                 proc p {} {\n\
                 set l orig\n\
                 trace add variable l write mang\n\
                 puts \"set-local: <[set l new]>|$l\"\n\
                 set m orig\n\
                 trace add variable m write mang\n\
                 puts \"append-local: <[append m x]>|$m\"\n\
                 set n orig\n\
                 trace add variable n write mang\n\
                 puts \"lappend-local: <[lappend n x]>|$n\"\n\
                 set o 1\n\
                 trace add variable o write bump\n\
                 puts \"incr-local: <[incr o]>|$o\"\n\
                 array set q {k orig}\n\
                 trace add variable q(k) write mange\n\
                 puts \"set-elem-local: <[set q(k) new]>|$q(k)\"\n\
                 puts \"lappend-elem-local: <[lappend q(k) z]>|$q(k)\"\n\
                 set r orig\n\
                 trace add variable r write mang\n\
                 puts \"lappend2-local: <[lappend r a b]>|$r\"\n\
                 }\n\
                 proc bump {n1 n2 op} { upvar 1 $n1 v; set v 100 }\n\
                 p\n\
                 set s orig\n\
                 trace add variable s write mang\n\
                 set nm s\n\
                 puts \"set-stk: <[set $nm new]>|$s\"\n",
        want: "append-top: <mangled>|mangled\n\
               lappend-top: <mangled>|mangled\n\
               set-elem: <mangled>|mangled\n\
               append-elem: <mangled>|mangled\n\
               lappend-elem: <mangled>|mangled\n\
               set-local: <mangled>|mangled\n\
               append-local: <mangled>|mangled\n\
               lappend-local: <mangled>|mangled\n\
               incr-local: <100>|100\n\
               set-elem-local: <mangled>|mangled\n\
               lappend-elem-local: <mangled>|mangled\n\
               lappend2-local: <mangled>|mangled\n\
               set-stk: <mangled>|mangled",
    },
    // Row 3: `incr` reads through `TclPtrGetVarIdx`, so the read trace fires
    // first — on a compiled slot, on a stack name, on an array element, and on
    // a variable the `incr` itself creates. `append` never fires `read`.
    Vector {
        name: "incr fires read before write; append and plain set fire write only",
        script: "proc R {n1 n2 op} { lappend ::log $op }\n\
                 proc RE {n1 n2 op} { lappend ::log $op:$n1,$n2 }\n\
                 set x 1\n\
                 trace add variable x {read write} R\n\
                 set ::log {}\n\
                 incr x\n\
                 puts \"incr: $::log\"\n\
                 set ::log {}\n\
                 incr x 5\n\
                 puts \"incr5: $::log\"\n\
                 set ::log {}\n\
                 append x a\n\
                 puts \"append: $::log\"\n\
                 set ::log {}\n\
                 set x 3\n\
                 puts \"set: $::log\"\n\
                 trace add variable nx {read write} R\n\
                 set ::log {}\n\
                 incr nx\n\
                 puts \"incr-new: $::log nx=$nx\"\n\
                 array set a {k 1}\n\
                 trace add variable a(k) {read write} RE\n\
                 set ::log {}\n\
                 incr a(k)\n\
                 puts \"incr-elem: $::log\"\n\
                 proc p1 {} {\n\
                 set l 1\n\
                 trace add variable l {read write} R\n\
                 set ::log {}\n\
                 incr l\n\
                 puts \"incr-local: $::log\"\n\
                 set ::log {}\n\
                 incr l 2\n\
                 puts \"incr-local2: $::log\"\n\
                 }\n\
                 p1\n",
        want: "incr: read write\n\
               incr5: read write\n\
               append: write\n\
               set: write\n\
               incr-new: read write nx=1\n\
               incr-elem: read:a,k write:a,k\n\
               incr-local: read write\n\
               incr-local2: read write",
    },
    // Row 4: `lappend` fires `read` only where C reaches `TclPtrGetVarIdx` —
    // the dispatched `Tcl_LappendObjCmd` (everything outside a proc body, and
    // the no-value form) and the multi-value `INST_LAPPEND_LIST*` opcodes
    // (`tclExecute.c:3391`). C's single-value in-proc opcodes
    // `INST_LAPPEND_{SCALAR,ARRAY,STK,ARRAY_STK}` omit `TCL_TRACE_READS`
    // (`tclExecute.c:3110-3121`) and fire `write` only, and so do this VM's.
    //
    // The in-proc single-value spellings (`proc-local1`, `proc-elem1`) reach
    // the write-only opcodes and are on this sheet: `cmd_proc` used to look the
    // pre-compiled body up under the *unqualified* `reg_name` while the
    // compiler keys module procedures by `::name`, so a global proc always
    // missed and its body was recompiled as a top-level script, losing every
    // `is_proc` specialisation. Rooting that lookup made both correct.
    //
    // Residual, deliberately absent from this sheet: an in-proc
    // `eval {lappend l z}`. C has no `eval` compiler — the script becomes its
    // own unit, where `l` is not a compiled local, so `lappend` dispatches and
    // fires `read write`. This compiler *relaxes* a literal `eval` body into
    // the enclosing function (`try_lower_eval_static` → `Statement::Block`),
    // and the CFG builder flattens it, so inside a proc the body inherits the
    // proc's compiled locals and fires `write` alone. The divergence is the
    // relaxation, not the trace code, and it predates the lookup fix — a
    // script-shaped proc body simply masked it. Closing it means carrying
    // eval-body provenance through the CFG to codegen, or not relaxing `eval`
    // inside a proc at all.
    Vector {
        name: "lappend fires read on the dispatched and multi-value paths only",
        script: "proc R {n1 n2 op} { lappend ::log $op }\n\
                 set x a\n\
                 trace add variable x {read write} R\n\
                 set ::log {}\n\
                 lappend x z\n\
                 puts \"top-1: $::log\"\n\
                 set ::log {}\n\
                 lappend x\n\
                 puts \"top-0: $::log\"\n\
                 set ::log {}\n\
                 lappend x a b\n\
                 puts \"top-2: $::log\"\n\
                 set g a\n\
                 trace add variable g {read write} R\n\
                 set ::log {}\n\
                 eval {lappend g z}\n\
                 puts \"top-eval: $::log\"\n\
                 set ::log {}\n\
                 if 1 {lappend g z}\n\
                 puts \"top-if: $::log\"\n\
                 proc p {} {\n\
                 set l a\n\
                 trace add variable l {read write} R\n\
                 set ::log {}\n\
                 lappend l a b\n\
                 puts \"proc-local2: $::log\"\n\
                 set ::log {}\n\
                 lappend l\n\
                 puts \"proc-local0: $::log\"\n\
                 set ::log {}\n\
                 lappend l z\n\
                 puts \"proc-local1: $::log\"\n\
                 array set arr {k a}\n\
                 trace add variable arr(k) {read write} R\n\
                 set ::log {}\n\
                 lappend arr(k) a b\n\
                 puts \"proc-elem2: $::log\"\n\
                 set ::log {}\n\
                 lappend arr(k) z\n\
                 puts \"proc-elem1: $::log\"\n\
                 }\n\
                 p\n",
        want: "top-1: read write\n\
               top-0: read\n\
               top-2: read write\n\
               top-eval: read write\n\
               top-if: read write\n\
               proc-local2: read write\n\
               proc-local0: read\n\
               proc-local1: write\n\
               proc-elem2: read write\n\
               proc-elem1: write",
    },
    // Rows 3 + 4: the read a read-modify-write command performs treats a
    // trace error as "no current value" rather than as a failure — `incr`
    // counts from 0, `lappend` discards the old value — and the swallowed
    // error stays logged in `::errorInfo` with its `(read trace on "x")`
    // frame and no `invoked from within` for the surviving command.
    Vector {
        name: "an erroring read trace leaves incr and lappend succeeding, error logged",
        script: "proc boom {n1 n2 op} { error bang }\n\
                 set x 1\n\
                 trace add variable x read boom\n\
                 set c [catch {incr x} m]\n\
                 set ei $::errorInfo\n\
                 trace remove variable x read boom\n\
                 puts \"incr: code=$c msg=$m x=$x\"\n\
                 puts \"ei-tail: [lrange [split $ei \\n] end-1 end]\"\n\
                 set y old\n\
                 trace add variable y read boom\n\
                 set c2 [catch {lappend y z} m2]\n\
                 trace remove variable y read boom\n\
                 puts \"lappend: code=$c2 msg=$m2 y=$y\"\n\
                 set z2 old\n\
                 trace add variable z2 read boom\n\
                 set c3 [catch {lappend z2} m3]\n\
                 trace remove variable z2 read boom\n\
                 puts \"lappend0: code=$c3 msg=$m3 z2=$z2\"\n",
        want: "incr: code=0 msg=1 x=1\n\
               ei-tail: {\"boom x {} read\"} {    (read trace on \"x\")}\n\
               lappend: code=0 msg=z y=z\n\
               lappend0: code=0 msg= z2=",
    },
    // Row 8 (variable): the firing walk consults the live trace list, not a
    // snapshot. C's `TclCallVarTraces` follows `active.nextTracePtr`
    // (`tclTrace.c` 9.0.4:2583, :2622) and `Tcl_UntraceVar2` unlinks a record
    // at once, rewriting every active walk (:2824-2842); `TraceVarEx` prepends,
    // so an addition sits behind the walk. A trace a callback removes therefore
    // does not fire in that pass — whether it is older and not yet reached, or
    // newer and already run — one it adds does not fire until the next access,
    // and `trace info` reflects both immediately.
    Vector {
        name: "a callback's trace remove and trace add are honoured mid-firing",
        script: "proc t1 {n1 n2 op} { trace remove variable ::t write t1\n\
                 puts \"t1 info: [trace info variable ::t]\" }\n\
                 proc t2 {n1 n2 op} { trace add variable ::t write t3\n\
                 puts \"t2 info: [trace info variable ::t]\" }\n\
                 proc t3 {n1 n2 op} { puts \"t3 fired\" }\n\
                 proc t4 {n1 n2 op} { puts \"t4 fired\" }\n\
                 set t 0\n\
                 trace add variable t write t4\n\
                 trace add variable t write t2\n\
                 trace add variable t write t1\n\
                 set t 1\n\
                 puts \"after: [trace info variable ::t]\"\n\
                 set t 2\n\
                 puts ---\n\
                 proc u1 {n1 n2 op} { trace remove variable ::u write u2\n\
                 puts \"u1 info: [trace info variable ::u]\" }\n\
                 proc u2 {n1 n2 op} { puts \"u2 fired\" }\n\
                 set u 0\n\
                 trace add variable u write u2\n\
                 trace add variable u write u1\n\
                 set u 1\n\
                 puts ---\n\
                 proc v1 {n1 n2 op} { puts \"v1 fired\" }\n\
                 proc v2 {n1 n2 op} { trace remove variable ::v write v1\n\
                 puts \"v2 info: [trace info variable ::v]\" }\n\
                 set v 0\n\
                 trace add variable v write v2\n\
                 trace add variable v write v1\n\
                 set v 1\n\
                 puts ---\n\
                 proc w1 {n1 n2 op} { trace remove variable ::w write w1\n\
                 trace add variable ::w write w1\n\
                 puts \"w1 info: [trace info variable ::w]\" }\n\
                 set w 0\n\
                 trace add variable w write w1\n\
                 set w 1\n",
        want: "t1 info: {write t2} {write t4}\n\
               t2 info: {write t3} {write t2} {write t4}\n\
               t4 fired\n\
               after: {write t3} {write t2} {write t4}\n\
               t3 fired\n\
               t2 info: {write t3} {write t3} {write t2} {write t4}\n\
               t4 fired\n\
               ---\n\
               u1 info: {write u1}\n\
               ---\n\
               v1 fired\n\
               v2 info: {write v2}\n\
               ---\n\
               w1 info: {write w1}",
    },
    // Row 10: `UnsetVarStruct` (`tclVar.c` 9.0.4:2560-2745) marks the variable
    // undefined and moves its trace list aside *before* the callbacks run, then
    // frees whatever list is left. So a callback sees the variable already
    // gone, a value it stores survives the unset, and the revived variable
    // carries no traces — not even the write trace that would otherwise have
    // fired on that store. Whole-array traces are not the element's, so an
    // element unset leaves them in place.
    //
    // Row 8's unset variant rides along: an unset callback that removes another
    // unset trace finds nothing to remove (the list is already out of the
    // table, C's Bug 3062331), and the taken list still fires it.
    Vector {
        name: "an unset takes the traces out before firing, so a revive survives trace-less",
        script: "proc rev {n1 n2 op} { puts \"in-trace exists=[info exists ::a]\"\n\
                 set ::a revived }\n\
                 set a orig\n\
                 trace add variable a unset rev\n\
                 unset a\n\
                 puts \"exists=[info exists a] val=<$a> traces=<[trace info variable a]>\"\n\
                 puts ---\n\
                 proc reve {n1 n2 op} { set ::b(k) revived }\n\
                 array set b {k v}\n\
                 trace add variable b(k) unset reve\n\
                 unset b(k)\n\
                 puts \"b exists=[info exists b(k)] val=<$b(k)> traces=<[trace info variable b(k)]>\"\n\
                 puts ---\n\
                 proc reva {n1 n2 op} { set ::c(k) revived }\n\
                 array set c {k v}\n\
                 trace add variable c unset reva\n\
                 unset c(k)\n\
                 puts \"c exists=[info exists c(k)] val=<$c(k)> traces on c=<[trace info variable c]>\"\n\
                 puts ---\n\
                 proc revw {n1 n2 op} { set ::d(j) revived }\n\
                 array set d {k v}\n\
                 trace add variable d unset revw\n\
                 unset d\n\
                 puts \"d exists=[info exists d(j)] val=<$d(j)> isarray=[array exists d] traces=<[trace info variable d]>\"\n\
                 puts ---\n\
                 proc revz {n1 n2 op} { set ::z revived }\n\
                 proc wz {n1 n2 op} { puts \"write fired\" }\n\
                 set z orig\n\
                 trace add variable z unset revz\n\
                 trace add variable z write wz\n\
                 unset z\n\
                 puts \"z=<$z> traces=<[trace info variable z]>\"\n\
                 puts ---\n\
                 proc revu {n1 n2 op} { set ::y revived\n\
                 puts \"y=$::y\"\n\
                 unset ::y\n\
                 puts \"exists=[info exists ::y]\" }\n\
                 set y orig\n\
                 trace add variable y unset revu\n\
                 unset y\n\
                 puts \"after exists=[info exists y]\"\n\
                 puts ---\n\
                 proc z1 {n1 n2 op} { trace remove variable ::zz unset z2\n\
                 puts \"z1 info: <[trace info variable ::zz]>\" }\n\
                 proc z2 {n1 n2 op} { puts \"z2 fired\" }\n\
                 set zz 0\n\
                 trace add variable zz unset z2\n\
                 trace add variable zz unset z1\n\
                 unset zz\n",
        want: "in-trace exists=0\n\
               exists=1 val=<revived> traces=<>\n\
               ---\n\
               b exists=1 val=<revived> traces=<>\n\
               ---\n\
               c exists=1 val=<revived> traces on c=<{unset reva}>\n\
               ---\n\
               d exists=1 val=<revived> isarray=1 traces=<>\n\
               ---\n\
               z=<revived> traces=<>\n\
               ---\n\
               y=revived\n\
               exists=0\n\
               after exists=0\n\
               ---\n\
               z1 info: <>\n\
               z2 fired",
    },
];

#[test]
fn vm_matches_the_pinned_trace_vectors() {
    for v in VECTORS {
        assert_eq!(vm_output(v.script), v.want, "{}", v.name);
    }
}

/// The table itself is pinned to C Tcl (8.6.16 and 9.0.4 agree on every line).
#[test]
fn vectors_match_real_tclsh() {
    let mut ran = 0;
    for v in VECTORS {
        for (env, names) in [
            ("TCL_LSP_TCLSH86", &["tclsh8.6"][..]),
            ("TCL_LSP_TCLSH90", &["tclsh9.0"][..]),
        ] {
            if let Some(got) = tclsh_output(env, names, v.script) {
                assert_eq!(got, v.want, "[{env}] {}", v.name);
                ran += 1;
            }
        }
    }
    if ran == 0 {
        eprintln!("skipping: neither tclsh8.6 nor tclsh9.0 found");
    }
}
