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

use tcl_compiler::compile_service::BytecodeCompileService;
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

/// Run `src` in the VM; the script's `puts` output is returned.
fn vm_output(src: &str) -> String {
    let service = BytecodeCompileService::default();
    let asm = service.compile(src).expect("test script compiles");

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(service));
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "VM run failed: {}",
        completion.result.to_str()
    );
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
    // `proc-eval` is the one that separates "is a proc body" from "is a
    // compiled local". C has no `eval` compiler: the script becomes its own
    // unit, where `l` is not a compiled local, so `lappend` dispatches and
    // fires `read write` — even though the enclosing frame is a proc. This
    // compiler *relaxes* a literal `eval` body into the enclosing function
    // (`try_lower_eval_static` → `Statement::Block`, flattened by the CFG
    // builder), so codegen asks `CodegenCtx::compiles_locals` — which excludes
    // the folded body's span — rather than `is_proc`.
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
                 set ::log {}\n\
                 eval {lappend l z}\n\
                 puts \"proc-eval: $::log\"\n\
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
               proc-eval: read write\n\
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

#[test]
fn array_operations_fire_the_registry_resolved_array_trace() {
    // Exact Tcl 9.0.4 transcript. This covers all seven members implemented
    // and advertised by this VM adapter; Tcl 9's `default` is not implemented
    // by the adapter yet.
    // Wrong arity and array-for's malformed variable list fail before
    // LocateArray; callback errors retain their Tcl message and error code.
    let script = r"
set events {}
proc A {tag n1 n2 op} {lappend ::events [list $tag $n1 $n2 $op]}
array set exists {x 1}
trace add variable exists array {A exists}
array exists exists
array set forvar {x 1}
trace add variable forvar array {A for}
array for {k v} forvar {}
array set get {x 1}
trace add variable get array {A get}
array get get
array set names {x 1}
trace add variable names array {A names}
array names names
array set setvar {x 1}
trace add variable setvar array {A set}
array set setvar {y 2}
array set size {x 1}
trace add variable size array {A size}
array size size
array set unsetvar {x 1}
trace add variable unsetvar array {A unset}
array unset unsetvar nomatch
set before $events
catch {array exists exists extra} m
set wrong [expr {$before eq $events}]
catch {array for x forvar {}} fm
set forbad [expr {$before eq $events}]
puts [list $events $wrong $forbad $m $fm]
array set a {x 1}
proc B {n1 n2 op} {return -code error -errorcode {APP ARRAY} boom}
trace add variable a array B
catch {array exists a} em eo
puts [list $em [dict get $eo -errorcode]]
";
    assert_eq!(
        vm_output(script),
        "{{exists exists {} array} {for forvar {} array} {get get {} array} {names names {} array} {set setvar {} array} {size size {} array} {unset unsetvar {} array}} 1 1 {wrong # args: should be \"array exists arrayName\"} {must have two variable names}\n{can't trace array \"a\": boom} {APP ARRAY}"
    );
}

#[test]
fn read_and_write_trace_wrappers_publish_their_lookup_error_codes() {
    // Exact Tcl 9.0.4 transcript. Unlike the array operation, scalar
    // read/write wrapping replaces the callback's code with the variable
    // lookup class while still committing a write before its callback.
    let script = r"
proc R {n1 n2 op} {return -code error -errorcode {APP READ} nope}
set x 1
trace add variable x read R
catch {set x} rm ro
proc W {n1 n2 op} {return -code error -errorcode {APP WRITE} bad}
set y 1
trace add variable y write W
catch {set y 2} wm wo
puts [list $rm [dict get $ro -errorcode] $wm [dict get $wo -errorcode] $y]
";
    assert_eq!(
        vm_output(script),
        "{can't read \"x\": nope} {TCL READ VARNAME x} {can't set \"y\": bad} {TCL WRITE VARNAME y} 2"
    );
}

#[test]
fn array_trace_resolution_handles_undefined_scalar_and_alias_cells() {
    // Exact Tcl 9.0.4 transcript. LocateArray reaches an undefined traced
    // shell, rejects a scalar without tracing, and reports an upvar's spelling
    // while retaining the target array's stable trace identity.
    let script = r"
set events {}
proc A {tag n1 n2 op} {lappend ::events [list $tag $n1 $n2 $op]}
trace add variable absent array {A absent}
set ae [array exists absent]
set scalar value
trace add variable scalar array {A scalar}
set se [array exists scalar]
proc aliasprobe {} {upvar #0 target a; return [array size a]}
array set target {x 1}
trace add variable target array {A alias}
set av [aliasprobe]
puts [list $ae $se $av $events]
";
    assert_eq!(
        vm_output(script),
        "0 0 1 {{absent absent {} array} {alias a {} array}}"
    );
}

#[test]
fn array_trace_resolution_distinguishes_direct_and_linked_elements() {
    // Exact Tcl 9.0.4 transcript. LocateArray gates on the reached cell's own
    // trace and state. A direct undefined element walks parent then element,
    // while an alias reaches only that element; scalar elements and an
    // untraced element under a traced parent do not fire.
    let script = r"
set events {}
proc A {tag n1 n2 op} {lappend ::events [list $tag $n1 $n2 $op]}
array set a {}
trace add variable a array {A parent}
trace add variable a(k) array {A elem}
set direct [array exists a(k)]
set directevents $events
set events {}
proc aliasprobe {} {upvar #0 a(k) e; array exists e}
set alias [aliasprobe]
set aliasevents $events
set events {}
set a(k) value
set defined [array exists a(k)]
set definedevents $events
array set b {}
trace add variable b array {A onlyparent}
set parentonly [array exists b(k)]
puts [list $direct $directevents $alias $aliasevents $defined $definedevents $parentonly $events]
";
    assert_eq!(
        vm_output(script),
        "0 {{parent a k array} {elem a k array}} 0 {{elem e k array}} 0 {} 0 {}"
    );
}

#[test]
fn array_trace_alias_retarget_uses_located_cell_policy() {
    // Tcl 9.0.4's LocateArray retains the original cell for key enumeration,
    // while set and whole-array unset deliberately re-resolve the spelling.
    // `get` combines the two (old keys, live values), patterned unset stays on
    // the located cell, and `for` detects a changed search identity.
    let script = r"
proc R {target n1 n2 op} {uplevel 1 [list upvar #0 $target x]}
array set as {a 1}
array set bs {b1 1 b2 2}
proc PS {} {
    upvar #0 as x
    trace add variable x array {R bs}
    list [array size x] [array size x]
}
set size [PS]
array set an {a 1}
array set bn {b1 1 b2 2}
proc PN {} {
    upvar #0 an x
    trace add variable x array {R bn}
    list [array exists x] [lsort [array names x]] [array size x]
}
set names [PN]
array set ag {a 1}
array set bg {b 2}
proc PG {} {
    upvar #0 ag x
    trace add variable x array {R bg}
    array get x
}
set get [PG]
array set aset {a 1}
array set bset {b 2}
proc PSET {} {
    upvar #0 aset x
    trace add variable x array {R bset}
    array set x {new 3}
}
PSET
set setrow [list [info exists aset(new)] [info exists bset(new)]]
array set au {a 1}
array set bu {b 2}
proc PU {} {
    upvar #0 au x
    trace add variable x array {R bu}
    array unset x
}
PU
set unsetrow [list [array exists au] [array exists bu]]
array set ap {a 1}
array set bp {b 2}
proc PP {} {
    upvar #0 ap x
    trace add variable x array {R bp}
    array unset x a
}
PP
set pattern [list [array size ap] [array size bp]]
array set af {a 1}
array set bf {b 2}
proc PF {} {
    upvar #0 af x
    trace add variable x array {R bf}
    catch {array for {k v} x {}} msg
    list $msg
}
set forrow [PF]
puts [list $size $names $get $setrow $unsetrow $pattern $forrow]
";
    assert_eq!(
        vm_output(script),
        "{1 2} {1 {b1 b2} 2} {} {0 1} {1 0} {0 1} {{array changed during iteration}}"
    );
}

#[test]
fn whole_array_unset_retargets_live_alias_and_preserves_const_error() {
    // Exact Tcl 9.0.4 transcript. LocateArray retains `old` for the array
    // operation, but whole-array mutation re-resolves the live spelling after
    // its array trace retargets `x`. The scalar constant rejects that unset;
    // both it and the originally located array remain unchanged.
    let script = r"
proc RCONST {n1 n2 op} {uplevel 1 {upvar #0 c x}}
array set old {k v}
const c locked
proc P {} {
    upvar #0 old x
    trace add variable x array RCONST
    set code [catch {array unset x} message options]
    list $code $message [dict get $options -errorcode] [array get ::old] $::c
}
puts [P]
";
    assert_eq!(
        vm_output(script),
        r#"1 {can't unset "x": variable is a constant} {TCL UNSET CONST} {k v} locked"#
    );
}

#[test]
fn array_operation_reference_preserves_same_binding_recreation() {
    // Tcl's LocateArray operation reference keeps the direct binding shell,
    // not merely the allocation, so a callback's same-name recreation refills
    // the cell observed by both command and compiler-opcode paths. The target
    // binding is retained through an alias too, nested operations release only
    // the final reference, and final cleanup never removes a replacement array
    // element. A scalar recreation is not removed by whole-array `unset`.
    let script = r"
proc R {n1 n2 op} {
    trace remove variable ::a array R
    unset ::a
    array set ::a {new1 2 new2 3}
}
array set a {old 1}
trace add variable a array R
set direct [list [lsort [array names a]] [lsort [array names a]]]
proc RI {n1 n2 op} {
    uplevel 1 [list trace remove variable $n1 array RI]
    uplevel 1 [list unset $n1]
    uplevel 1 [list array set $n1 {x 1}]
}
proc compiled {} {
    array set local {old 1}
    trace add variable local array RI
    list [array exists local] [lsort [array names local]]
}
proc RS {n1 n2 op} {
    trace remove variable ::s array RS
    unset ::s
    set ::s scalar
}
array set s {old 1}
trace add variable s array RS
array unset s
array set aa {old 1}
array set bb {new 2}
proc RA {n1 n2 op} {
    trace remove variable ::aa array RA
    uplevel 1 {upvar #0 bb x}
    unset ::aa
}
proc PA {} {
    upvar #0 aa x
    trace add variable x array RA
    list [array names x] [info vars ::aa] [array names x]
}
set aliasrow [PA]
array set nestedA {old 1}
set nestedResult unset
proc RN {n1 n2 op} {
    trace remove variable ::nestedA array RN
    unset ::nestedA
    set ::nestedResult [array exists ::nestedA]
    array set ::nestedA {new 2}
}
trace add variable nestedA array RN
set nestedrow [list [array names nestedA] [array names nestedA] $nestedResult]
array set elem {}
upvar #0 elem keep
proc RELEM {n1 n2 op} {
    trace remove variable ::elem(missing) array RELEM
    unset ::elem
    array set ::elem {missing fresh}
}
trace add variable elem(missing) array RELEM
set elemrow [list [array exists elem(missing)] [array get elem] [array get keep]]
puts [list $direct [compiled] [info exists s] [array exists s] $s \
    $aliasrow $nestedrow $elemrow]
";
    assert_eq!(
        vm_output(script),
        "{{new1 new2} {new1 new2}} {1 x} 1 0 scalar {{} {} new} {new new 0} {0 {missing fresh} {missing fresh}}"
    );
}

#[test]
fn array_for_tracks_structural_revision_and_read_trace_deletion() {
    // A delete/recreate of the same key invalidates Tcl's active search even
    // though the final key set is unchanged. A read trace that deletes the
    // current value still leaves the key assigned, preserves the prior value
    // variable, and runs the body once before that invalidation is reported.
    // Defining a physical-but-undefined trace/link shell does not invalidate
    // the search and makes that candidate visible if it has not been visited.
    let script = r#"
array set a {x 1}
set churnCode [catch {array for {k v} a {unset a($k); set a($k) 2}} churnMsg churnOpts]
array set b {x 1}
set k oldk
set v oldv
set events {}
proc RD {n1 n2 op} {
    trace remove variable ::b($n2) read RD
    unset ::b($n2)
}
trace add variable b(x) read RD
set readCode [catch {array for {k v} b {lappend ::events [list $k $v]}} readMsg readOpts]
array set c {x 1}
set errEvents {}
set ek oldk
set ev oldv
proc RE {n1 n2 op} {error boom}
trace add variable c(x) read RE
set errCode [catch {array for {ek ev} c {lappend ::errEvents [list $ek $ev]}} errMsg]
set keptErrorInfo [string match {*read trace on "c(x)"*} $::errorInfo]
proc N args {}
array set d {x 1}
trace add variable d(y) read N
set dEvents {}
set dCode [catch {array for {dk dv} d {
    lappend ::dEvents [list $dk $dv]
    if {$dk eq "x"} {set d(y) 2}
}} dMsg]
array set e {x 1}
upvar #0 e(y) eAlias
set eEvents {}
set eCode [catch {array for {ekey eval} e {
    lappend ::eEvents [list $ekey $eval]
    if {$ekey eq "x"} {set eAlias 2}
}} eMsg]
array set f {x 1}
upvar #0 f(y) fAlias
set fCode [catch {array for {fkey fval} f {
    if {$fkey eq "x"} {unset -nocomplain f(y)}
}} fMsg]
array set g {x 1}
upvar #0 g(y) gAlias
set gCode [catch {array for {gkey gval} g {
    if {$gkey eq "x"} {unset -nocomplain gAlias}
}} gMsg]
proc RSEARCH {n1 n2 op} {trace remove variable ::h(y) array RSEARCH}
array set h {x 1}
trace add variable h(y) array RSEARCH
set hEvents {}
set hCode [catch {array for {hkey hval} h {
    lappend ::hEvents [list $hkey $hval]
    if {$hkey eq "x"} {array exists h(y); set h(y) 2}
}} hMsg]
array set i {x 1}
trace add variable i(y) read N
trace remove variable i(y) read N
set iEvents {}
set iCode [catch {array for {ikey ival} i {
    lappend ::iEvents [list $ikey $ival]
    if {$ikey eq "x"} {set i(y) 2}
}} iMsg iOpts]
proc RKEEP {n1 n2 op} {trace remove variable ::j(y) array RKEEP}
array set j {x 1}
trace add variable j(y) array RKEEP
array for {jkey jval} j {
    if {$jkey eq "x"} {array exists j(y)}
}
set jEvents {}
set jCode [catch {array for {jkey jval} j {
    lappend ::jEvents [list $jkey $jval]
    if {$jkey eq "x"} {set j(y) 2}
}} jMsg]
array set z {x 1}
trace add variable z(y) read N
set zCode1 [catch {array for {zkey zval} z {
    if {$zkey eq "x"} {trace remove variable z(y) read N}
}} zMsg1]
set zEvents {}
set zCode2 [catch {array for {zkey zval} z {
    lappend ::zEvents [list $zkey $zval]
    if {$zkey eq "x"} {set z(y) 2}
}} zMsg2 zOpts2]
puts [list $churnCode $churnMsg [dict get $churnOpts -errorcode] [array get a] \
    $readCode $readMsg [dict get $readOpts -errorcode] $events $k $v \
    $errCode $errMsg $errEvents $ek $ev $keptErrorInfo \
    $dCode $dMsg $dEvents [array get d] $eCode $eMsg $eEvents [array get e] \
    $fCode $fMsg [array get f] $gCode $gMsg [array get g] \
    $hCode $hMsg $hEvents [array get h] $iCode $iMsg \
    [dict get $iOpts -errorcode] $iEvents [array get i] \
    $jCode $jMsg $jEvents [array get j] $zCode1 $zMsg1 $zCode2 $zMsg2 \
    [dict get $zOpts2 -errorcode] $zEvents [array get z]]
"#;
    assert_eq!(
        vm_output(script),
        "1 {array changed during iteration} {TCL READ array for} {x 2} 1 {array changed during iteration} {TCL READ array for} {{x oldv}} x oldv 0 {} {{x oldv}} x oldv 1 0 {} {{x 1} {y 2}} {x 1 y 2} 0 {} {{x 1} {y 2}} {x 1 y 2} 1 {array changed during iteration} {x 1} 1 {array changed during iteration} {x 1} 0 {} {{x 1} {y 2}} {x 1 y 2} 1 {array changed during iteration} {TCL READ array for} {{x 1}} {x 1 y 2} 0 {} {{x 1} {y 2}} {x 1 y 2} 0 {} 1 {array changed during iteration} {TCL READ array for} {{x 1}} {x 1 y 2}"
    );
}

#[test]
fn array_for_non_array_errors_have_the_lookup_identity() {
    let script = r"
set missingCode [catch {array for {k v} missing {}} missingMsg missingOpts]
set scalar 1
set scalarCode [catch {array for {k v} scalar {}} scalarMsg scalarOpts]
proc RSF {n1 n2 op} {
    trace remove variable ::traced array RSF
    unset ::traced
    set ::traced scalar
}
array set traced {x 1}
trace add variable traced array RSF
set tracedCode [catch {array for {k v} traced {}} tracedMsg tracedOpts]
puts [list $missingCode $missingMsg [dict get $missingOpts -errorcode] \
    $scalarCode $scalarMsg [dict get $scalarOpts -errorcode] \
    $tracedCode $tracedMsg [dict get $tracedOpts -errorcode]]
";
    assert_eq!(
        vm_output(script),
        "1 {\"missing\" isn't an array} {TCL LOOKUP ARRAY missing} 1 {\"scalar\" isn't an array} {TCL LOOKUP ARRAY scalar} 1 {\"traced\" isn't an array} {TCL LOOKUP ARRAY traced}"
    );
}

#[test]
fn whole_array_teardown_visits_trace_only_undefined_elements() {
    // TraceVarEx materialises an undefined element cell, and DeleteArray walks
    // that cell even though it has no value and is absent from `array names`.
    let script = r"
set events {}
proc U {n1 n2 op} {lappend ::events [list $n1 $n2 $op]}
array set a {}
trace add variable a(missing) unset U
unset a
puts $events
";
    assert_eq!(vm_output(script), "{a missing unset}");
}

#[test]
fn unsetting_a_trace_only_array_element_fires_then_reports_it_missing() {
    // Exact Tcl 9.0.4 transcript. A trace materialises the element's variable
    // cell, but does not give it a value: unset tears the cell down and runs
    // its callback before reporting that no element existed.
    let script = r"
proc U args {incr ::fired}
proc P {} {
    array set a {}
    trace add variable a(missing) unset U
    set code [catch {unset a(missing)} message options]
    list $code $message [dict get $options -errorcode]
}
set fired 0
puts [list [P] $fired]
";
    assert_eq!(
        vm_output(script),
        r#"{1 {can't unset "a(missing)": no such element in array} {TCL UNSET VARNAME}} 1"#
    );
}

#[test]
fn unset_element_miss_kind_survives_parent_mutation_in_its_callback() {
    // Exact Tcl 9.0.4 transcript. The undefined element is selected before its
    // callback runs, so deleting or retyping the parent cannot rewrite the
    // pending lookup failure.
    let script = r"
proc U {action args} {
    incr ::fired
    uplevel #0 $action
}
set fired 0
set outcomes {}
array set a {}
trace add variable a(missing) unset [list U {unset -nocomplain ::a}]
set code [catch {unset a(missing)} message options]
lappend outcomes [list $code $message [dict get $options -errorcode]]
array set b {}
trace add variable b(missing) unset [list U {unset -nocomplain ::b; set ::b scalar}]
set code [catch {unset b(missing)} message options]
lappend outcomes [list $code $message [dict get $options -errorcode]]
puts [list $outcomes $fired $b]
";
    assert_eq!(
        vm_output(script),
        r#"{{1 {can't unset "a(missing)": no such element in array} {TCL UNSET VARNAME}} {1 {can't unset "b(missing)": no such element in array} {TCL UNSET VARNAME}}} 2 scalar"#
    );
}

#[test]
fn every_advertised_array_member_validates_arity_before_tracing() {
    // Exact Tcl 9.0.4 transcript for all seven members implemented by the VM.
    // The registry arity check precedes LocateArray, so none reaches its trace.
    let script = r"
set events {}
proc A {tag n1 n2 op} {lappend ::events [list $tag $n1 $n2 $op]}
foreach n {exists forvar get names setvar size unsetvar} {
    array set $n {x 1}
    trace add variable $n array [list A $n]
}
set outcomes {}
lappend outcomes [catch {array exists exists extra}] [expr {$events eq {}}]
lappend outcomes [catch {array for {k v} forvar}] [expr {$events eq {}}]
lappend outcomes [catch {array get get p extra}] [expr {$events eq {}}]
lappend outcomes [catch {array names names -glob p extra}] [expr {$events eq {}}]
lappend outcomes [catch {array set setvar}] [expr {$events eq {}}]
lappend outcomes [catch {array size size extra}] [expr {$events eq {}}]
lappend outcomes [catch {array unset unsetvar p extra}] [expr {$events eq {}}]
puts $outcomes
";
    assert_eq!(vm_output(script), "1 1 1 1 1 1 1 1 1 1 1 1 1 1");
}

#[test]
fn compiled_proc_local_array_exists_fires_the_array_trace() {
    // Exact Tcl 9.0.4 transcript. A proc-local `array exists` lowers to the
    // dedicated ARRAY_EXISTS_IMM opcode, which must share array_op's trace
    // owner rather than query the local slot directly.
    let script = r"
set events {}
proc A {n1 n2 op} {lappend ::events [list $n1 $n2 $op]}
proc p {} {
    array set a {k v}
    trace add variable a array A
    array exists a
}
puts [list [p] $events]
";
    assert_eq!(vm_output(script), "1 {{a {} array}}");
}

#[test]
fn trace_reentrancy_is_per_resolved_variable_cell() {
    // Exact Tcl 9.0.4 transcript: entering a sibling element from a write
    // callback is not suppressed by the first element's active bit.
    let script = r#"
set events {}
proc E {tag n1 n2 op} {
    lappend ::events [list $tag $n1 $n2 $op]
    if {$tag eq "x"} {set ::a(y) nested}
}
set a(x) old
set a(y) old
trace add variable a(x) write {E x}
trace add variable a(y) write {E y}
set a(x) outer
puts $events
"#;
    assert_eq!(vm_output(script), "{x a x write} {y ::a y write}");
}

#[test]
fn whole_array_write_trace_can_enter_a_sibling_element() {
    // Exact Tcl 9.0.4 transcript. The parent trace list is selected for both
    // writes, but activity belongs to each resolved element cell, so x does
    // not suppress the nested write to y.
    let script = r#"
set events {}
proc W {n1 n2 op} {
    lappend ::events [list $n1 $n2 $op]
    if {$n2 eq "x"} {set ::c(y) nested}
}
array set c {x old y old}
trace add variable c write W
set c(x) outer
puts [list $c(x) $c(y) $events]
"#;
    assert_eq!(
        vm_output(script),
        "outer nested {{c x write} {::c y write}}"
    );
}

#[test]
fn array_operation_trace_self_gates_on_the_parent_cell() {
    // Exact Tcl 9.0.4 transcript. Array operations resolve to the parent cell,
    // so a callback's recursive operation on that same parent is suppressed.
    let script = r"
set events {}
proc P {n1 n2 op} {lappend ::events [list $n1 $n2 $op]; array size ::p}
array set p {x 1}
trace add variable p array P
puts [list [array exists p] $events]
";
    assert_eq!(vm_output(script), "1 {{p {} array}}");
}

#[test]
fn unset_from_an_active_write_trace_fires_the_taken_unset_list() {
    // Exact Tcl 9.0.4 transcript. Unset moves the trace list to a dummy cell
    // whose active bit is clear, so the nested unset callback still runs.
    let script = r#"
set events {}
proc W {n1 n2 op} {
    lappend ::events [list W $n1 $n2 $op]
    if {$op eq "write"} {unset ::a(k)}
}
trace add variable a(k) {write unset} W
set a(k) v
puts $events
"#;
    assert_eq!(vm_output(script), "{W a k write} {W ::a k unset}");
}

#[test]
fn whole_array_unset_fires_each_old_element_cell() {
    // Exact Tcl 9.0.4 transcript. The parent fires once, every old element
    // fires even when an earlier callback errors, and unset-trace errors do not
    // fail `unset`. Sorting removes only Tcl hash iteration order.
    let script = r#"
set events {}
proc U {tag n1 n2 op} {
    lappend ::events [list $tag $n1 $n2 $op]
    if {$tag eq "x"} {error boom}
}
array set a {x X y Y}
trace add variable a unset {U parent}
trace add variable a(x) unset {U x}
trace add variable a(y) unset {U y}
set code [catch {unset a} message]
puts [list $code $message [lsort $events] [info exists a]]
"#;
    assert_eq!(
        vm_output(script),
        "0 {} {{parent a {} unset} {x a x unset} {y a y unset}} 0"
    );
}

#[test]
fn whole_array_unset_detaches_old_element_aliases() {
    // Exact Tcl 9.0.4 transcript. The alias keeps the old element identity and
    // must not retarget the fresh same-name element created afterwards.
    let script = r"
set events {}
proc U {n1 n2 op} {lappend ::events [list $n1 $n2 $op]}
set x(k) old
upvar #0 x(k) alias
trace add variable x(k) unset U
unset x
set x(k) fresh
set code [catch {set alias rewritten} msg opts]
puts [list $events $code $msg [dict get $opts -errorcode] \
           [set x(k)] [trace info variable x(k)]]
";
    assert_eq!(
        vm_output(script),
        "{{x k unset}} 1 {can't set \"alias\": upvar refers to element in deleted array} {TCL WRITE VARNAME} fresh {}"
    );
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
