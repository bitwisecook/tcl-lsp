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

//! Issue #946 (M16.2 completion) — cross-interpreter alias re-entry parity.
//!
//! These vectors exercise the surfaces the M16.2 text previously documented as
//! divergences and the issue re-scoped as P1 faults to fix:
//!
//! * **fault 1** — a parent-target alias reached across a *native* re-entry
//!   (a resumed coroutine, an `lsort -command` comparator, a trace callback, an
//!   `after`/`vwait` event callback) used to error
//!   `cannot invoke parent-interp alias: C stack busy`.  C Tcl runs it; so does
//!   the VM now (the engine reaches an interpreter by swapping its arena-held
//!   state rather than by suspending an un-re-enterable Rust stack).
//! * **fault 2** — a parent alias target that *re-enters the child that
//!   invoked it* used to get `could not find interpreter`.  A suspended /
//!   executing interpreter stays addressable in the arena, so re-entry now
//!   follows C Tcl.
//!
//! Every vector is a complete script whose stdout is compared against the
//! bytecode VM **and** — when installed — real `tclsh8.6` / `tclsh9.0`
//! (`TCL_LSP_TCLSH86` / `TCL_LSP_TCLSH90` override the binaries), so the table
//! cannot drift from C Tcl.  The one deliberate *non*-divergence pinned here is
//! `yield` itself: a `yield` (not an alias call) still cannot cross a native
//! re-entry — C Tcl reports `cannot yield: C stack busy` and so does the VM.

use std::cell::RefCell;
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

/// Run `src` in the VM; the script's `puts` output is returned (trimmed).
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

/// One behaviour vector: the script prints its observations; `want` is the
/// full expected stdout (identical under 8.6 and 9.0 unless noted).
struct Vector {
    name: &'static str,
    script: &'static str,
    want: &'static str,
}

const VECTORS: &[Vector] = &[
    // -- fault 1: parent alias reached across a native re-entry ----------
    Vector {
        name: "TP: a parent alias fires from inside a resumed coroutine",
        script: "proc ptarget {x} { return parent:$x }\n\
                 interp create c\n\
                 interp alias c pfoo {} ptarget\n\
                 set r [interp eval c {\n\
                     proc co {} { yield; set v [pfoo fromcoro]; yield $v }\n\
                     coroutine k co\n\
                     k\n\
                 }]\n\
                 interp delete c\n\
                 puts $r\n",
        want: "parent:fromcoro",
    },
    Vector {
        name: "TP: a parent alias fires from an lsort -command comparator",
        script: "proc pcmp {a b} { string compare $a $b }\n\
                 interp create c\n\
                 interp alias c pc {} pcmp\n\
                 puts [interp eval c {lsort -command pc {b c a}}]\n\
                 interp delete c\n",
        want: "a b c",
    },
    Vector {
        name: "TP: a parent alias fires from a child variable-write trace",
        script: "proc ptgt {args} { return traced }\n\
                 interp create c\n\
                 interp alias c pt {} ptgt\n\
                 puts [interp eval c {\n\
                     set log {}\n\
                     trace add variable v write {apply {{n1 n2 op} { global log; set log [pt] }}}\n\
                     set v 1\n\
                     set log\n\
                 }]\n\
                 interp delete c\n",
        want: "traced",
    },
    Vector {
        name: "TP: a parent alias fires from a child after/vwait callback",
        script: "proc ptgt {} { return from-timer }\n\
                 interp create c\n\
                 interp alias c pt {} ptgt\n\
                 puts [interp eval c {\n\
                     set done {}\n\
                     after 0 { set done [pt] }\n\
                     vwait done\n\
                     set done\n\
                 }]\n\
                 interp delete c\n",
        want: "from-timer",
    },
    // -- fault 1: completion codes carried across the boundary ------------
    Vector {
        name: "TP: an error crossing a native re-entry propagates catchably with its code",
        script: "proc perr {} { error boom EI {ECODE X} }\n\
                 interp create c\n\
                 interp alias c pe {} perr\n\
                 puts [interp eval c {\n\
                     proc co {} { yield; catch { pe } m o; yield [list $m [dict get $o -errorcode] [dict get $o -code]] }\n\
                     coroutine k co\n\
                     k\n\
                 }]\n\
                 interp delete c\n",
        want: "boom {ECODE X} 1",
    },
    Vector {
        name: "TP: a break completion crosses the boundary as an error outside a loop",
        script: "proc pbrk {} { return -level 0 -code break }\n\
                 interp create c\n\
                 interp alias c pb {} pbrk\n\
                 puts [interp eval c { catch { pb } m o; list $m [dict get $o -code] }]\n\
                 interp delete c\n",
        want: "{invoked \"break\" outside of a loop} 1",
    },
    Vector {
        name: "TP: a custom completion code crosses the boundary intact",
        script: "proc pcust {} { return -level 0 -code 7 custom }\n\
                 interp create c\n\
                 interp alias c pc {} pcust\n\
                 puts [interp eval c { catch { pc } m o; list $m [dict get $o -code] }]\n\
                 interp delete c\n",
        want: "custom 7",
    },
    // -- fault 2: a parent target re-enters the child that called it -------
    Vector {
        name: "TP: a parent alias target re-enters the calling child",
        script: "interp create c\n\
                 proc ptarget {} { set r [interp eval c { expr {6*7} }]; return reentered:$r }\n\
                 interp alias c pfoo {} ptarget\n\
                 puts [interp eval c { pfoo }]\n\
                 interp delete c\n",
        want: "reentered:42",
    },
    Vector {
        name: "TP: a parent target re-enters the child and mutates its state visibly",
        script: "interp create c\n\
                 proc ptarget {} { interp eval c { set inner planted }; return ok }\n\
                 interp alias c pfoo {} ptarget\n\
                 set a [interp eval c { pfoo }]\n\
                 set b [interp eval c { set inner }]\n\
                 interp delete c\n\
                 puts $a-$b\n",
        want: "ok-planted",
    },
    Vector {
        name: "TP: a parent target that deletes the calling child mid-eval finishes the eval",
        script: "interp create c\n\
                 proc pkill {} { interp delete c; return killed }\n\
                 interp alias c pk {} pkill\n\
                 puts [interp eval c { list [pk] after }]\n\
                 puts [interp exists c]\n",
        want: "killed after\n0",
    },
    // -- fault 1 boundary: the still-native yield limit is preserved -------
    Vector {
        name: "TN: a bare yield still cannot cross a cross-interp callback (like C Tcl)",
        script: "proc pyield {} { yield marker }\n\
                 interp create c\n\
                 interp alias c py {} pyield\n\
                 proc co {} { yield; interp eval c { py }; puts after }\n\
                 puts [catch { coroutine k co; k } m]:$m\n\
                 interp delete c\n",
        want: "1:cannot yield: C stack busy",
    },
    // -- nested (grandchild) interpreters -----------------------------------
    Vector {
        name: "TP: a multi-word interp path routes an alias into a grandchild",
        script: "interp create a\n\
                 interp eval a { interp create b }\n\
                 interp eval {a b} { proc t {} { return grandchild } }\n\
                 interp alias {} hi {a b} t\n\
                 puts [hi]\n\
                 interp delete a\n",
        want: "grandchild",
    },
    Vector {
        name: "TP: a parent target re-enters a DIFFERENT child than its caller",
        script: "interp create c1\n\
                 interp create c2\n\
                 interp eval c2 { proc helper {} { return c2val } }\n\
                 proc ptgt {} { return [interp eval c2 { helper }] }\n\
                 interp alias c1 pfoo {} ptgt\n\
                 puts [interp eval c1 { pfoo }]\n\
                 interp delete c1\n\
                 interp delete c2\n",
        want: "c2val",
    },
    Vector {
        name: "TP: a coroutine three interpreters deep still reaches an alias to the root",
        script: "interp create a\n\
                 interp eval a { interp create b }\n\
                 proc roottgt {} { return from-root }\n\
                 interp alias {a b} rt {} roottgt\n\
                 puts [interp eval {a b} {\n\
                     proc co {} { yield; set v [rt]; yield $v }\n\
                     coroutine k co\n\
                     k\n\
                 }]\n\
                 interp delete a\n",
        want: "from-root",
    },
    // -- frame/interpreter identity across the boundary ---------------------
    Vector {
        name: "TP: a child-target alias's frame is the parent's, not the alias-call frame",
        script: "interp create c\n\
                 proc outer {} { return [interp eval c {pfoo}] }\n\
                 proc ptgt {} { list [info level] [uplevel 1 {info level}] }\n\
                 interp alias c pfoo {} ptgt\n\
                 puts [outer]\n\
                 puts [ptgt]\n\
                 interp delete c\n",
        want: "2 1\n1 0",
    },
    Vector {
        name: "FN: a same-interp alias never switches interps (the call frame stays put)",
        script: "proc probe {} { info level }\n\
                 interp alias {} p {} probe\n\
                 proc caller {} { p }\n\
                 puts [caller]\n",
        want: "2",
    },
    // -- sibling aliases (routed through the shared parent) ----------------
    Vector {
        name: "TP: a sibling→sibling alias dispatches into the other child",
        script: "interp create a\n\
                 interp create b\n\
                 interp eval b { proc greet {who} { return b-greets:$who } }\n\
                 interp alias a hi b greet\n\
                 puts [interp eval a { hi world }]\n\
                 interp delete a\n\
                 interp delete b\n",
        want: "b-greets:world",
    },
    Vector {
        name: "FP: a sibling alias loop is still refused",
        script: "interp create a\n\
                 interp create b\n\
                 interp alias a x b y\n\
                 puts [catch {interp alias b y a x} m]:$m\n\
                 interp delete a\n\
                 interp delete b\n",
        want: "1:cannot define or rename alias \"y\": would create a loop",
    },
    // -- interp lifecycle: alias identity across target death --------------
    Vector {
        name: "TP: deleting the target interp removes the alias from its source",
        script: "interp create a\n\
                 interp create b\n\
                 interp eval b { proc t {} { return bt } }\n\
                 interp alias a hi b t\n\
                 puts [interp eval a { hi }]\n\
                 interp delete b\n\
                 puts [catch {interp eval a { hi }} m]:$m\n\
                 puts n=[interp eval a { llength [info commands hi] }]\n\
                 interp delete a\n",
        want: "bt\n1:invalid command name \"hi\"\nn=0",
    },
    Vector {
        name: "TN: recreating a same-named target does not resurrect the old alias",
        script: "interp create b\n\
                 interp eval b { proc t {} { return old } }\n\
                 interp create a\n\
                 interp alias a hi b t\n\
                 interp delete b\n\
                 interp create b\n\
                 interp eval b { proc t {} { return new } }\n\
                 puts [catch {interp eval a { hi }} m]:$m\n\
                 interp delete a\n\
                 interp delete b\n",
        want: "1:invalid command name \"hi\"",
    },
    // -- safe children ----------------------------------------------------
    Vector {
        name: "TP: a safe child can still call a parent-target alias",
        script: "proc ptgt {} { return safe-parent-call }\n\
                 interp create -safe s\n\
                 interp alias s pt {} ptgt\n\
                 puts [interp eval s { pt }]\n\
                 interp delete s\n",
        want: "safe-parent-call",
    },
];

#[test]
fn vm_matches_the_pinned_reentry_vectors() {
    for v in VECTORS {
        assert_eq!(vm_output(v.script), v.want, "{}", v.name);
    }
}

/// The table is pinned to C Tcl: every vector's `want` must match what the real
/// tclsh prints (8.6 and 9.0 agree on all of these).  Skips per-binary when the
/// interpreter is not installed.
#[test]
fn reentry_vectors_match_real_tclsh() {
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
