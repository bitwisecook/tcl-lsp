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

//! Cross-version vectors for the deprecated `trace variable|vdelete|vinfo`
//! forms (issue #1444).
//!
//! C compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES`
//! (`tclTrace.c` 8.6.16:198-206); Tcl 9.0 dropped them, so the very same
//! script is a working legacy trace at 8.x and a `bad option` at 9.x. The
//! registry states that boundary (`DialectSet::TCL8X` on the three
//! subcommands), and the VM reads it rather than carrying its own list.
//!
//! The ops word is the `rwua` letter concatenation, expanded and validated by
//! `tcl_cmd_core::trace::parse_legacy_variable_ops` — so a non-`rwua` byte is
//! an error rather than a silently-installed never-firing trace, repeats
//! collapse, and the stored set is the same canonical set `trace add
//! variable` produces (`vdelete` therefore removes an `add`-installed trace).
//! `trace vinfo` renders that set back as letters in C's fixed `r`, `w`, `u`,
//! `a` order, unlike `trace info variable`'s word list.

use std::cell::RefCell;
use std::io::Write as _;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc;

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        self.compile_for_profile(src, DialectProfile::by_name(TclVersion::V9_0.dialect_name()))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::registry_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
            src,
            registry,
            config,
            profile.name,
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
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

fn vm_output(src: &str, version: TclVersion) -> String {
    let profile = DialectProfile::by_name(version.dialect_name());
    let asm = CompilerSvc
        .compile_for_profile(src, profile)
        .expect("test script compiles for its selected profile");
    let capture = Capture::default();
    let mut vm = Vm::with_output(Box::new(capture.clone()));
    vm.set_compiler(Box::new(CompilerSvc));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&capture.0.borrow())
        .trim()
        .to_owned()
}

fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    let mut candidates = std::env::var(bin_env).ok().into_iter().collect::<Vec<_>>();
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(name)
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

/// Runs to completion on every release: each line is `catch`-wrapped so the
/// 9.x `bad option` errors do not abort the script.
const SCRIPT: &str = "\
proc cb args { puts \"fired [lindex $args end]\" }\n\
proc cb2 args {}\n\
puts \"badopt: [catch {trace zzz} m]:$m\"\n\
puts \"wrongargs: [catch {trace variable x} m]:$m\"\n\
puts \"wrongargs2: [catch {trace vdelete x} m]:$m\"\n\
puts \"wrongargs3: [catch {trace vinfo x y} m]:$m\"\n\
puts \"badops: [catch {trace variable x q cb} m]:$m\"\n\
puts \"listops: [catch {trace vdelete x {read write} cb} m]:$m\"\n\
puts \"dedup: [catch {trace variable x rrw cb} m]:$m\"\n\
puts \"vinfo: [catch {trace vinfo x} m]:$m\"\n\
puts \"info: [catch {trace info variable x} m]:$m\"\n\
puts \"prefix: [catch {trace var x w cb2} m]:$m\"\n\
puts \"vinfo2: [catch {trace vinfo x} m]:$m\"\n\
puts \"crossremove: [catch {trace vdelete x wr cb} m]:$m\"\n\
puts \"vinfo3: [catch {trace vinfo x} m]:$m\"\n\
puts \"modernadd: [catch {trace add variable y {write read} cb} m]:$m\"\n\
puts \"legacyremove: [catch {trace vdelete y rw cb} m]:$m\"\n\
puts \"vinfo4: [catch {trace vinfo y} m]:$m\"\n\
puts \"all: [catch {trace variable z rwua cb} m]:$m\"\n\
puts \"vinfoall: [catch {trace vinfo z} m]:$m\"\n\
puts \"infoall: [catch {trace info variable z} m]:$m\"\n\
puts \"vivify: [info exists x][info exists z]\"\n\
puts \"legacyfire: [catch {set z 1} m]:$m\"\n\
trace add variable m2 write cb\n\
puts \"modernfire: [catch {set m2 1} m]:$m\"\n";

/// Tcl 8.4-8.6: the legacy forms work, and `trace var` abbreviates to
/// `trace variable` (C resolves the option word with `Tcl_GetIndexFromObj`
/// flags `0`).
const EXPECT_8X: &str = "\
badopt: 1:bad option \"zzz\": must be add, info, remove, variable, vdelete, or vinfo\n\
wrongargs: 1:wrong # args: should be \"trace variable name ops command\"\n\
wrongargs2: 1:wrong # args: should be \"trace vdelete name ops command\"\n\
wrongargs3: 1:wrong # args: should be \"trace vinfo name\"\n\
badops: 1:bad operations \"q\": should be one or more of rwua\n\
listops: 1:bad operations \"read write\": should be one or more of rwua\n\
dedup: 0:\n\
vinfo: 0:{rw cb}\n\
info: 0:{{read write} cb}\n\
prefix: 0:\n\
vinfo2: 0:{w cb2} {rw cb}\n\
crossremove: 0:\n\
vinfo3: 0:{w cb2}\n\
modernadd: 0:\n\
legacyremove: 0:\n\
vinfo4: 0:\n\
all: 0:\n\
vinfoall: 0:{rwua cb}\n\
infoall: 0:{{array read write unset} cb}\n\
fired r\n\
vivify: 00\n\
fired w\n\
legacyfire: 0:1\n\
fired write\n\
modernfire: 0:1";

/// Tcl 9.0+: `Tcl_TraceObjCmd`'s option table no longer carries them.
const EXPECT_9X: &str = "\
badopt: 1:bad option \"zzz\": must be add, info, or remove\n\
wrongargs: 1:bad option \"variable\": must be add, info, or remove\n\
wrongargs2: 1:bad option \"vdelete\": must be add, info, or remove\n\
wrongargs3: 1:bad option \"vinfo\": must be add, info, or remove\n\
badops: 1:bad option \"variable\": must be add, info, or remove\n\
listops: 1:bad option \"vdelete\": must be add, info, or remove\n\
dedup: 1:bad option \"variable\": must be add, info, or remove\n\
vinfo: 1:bad option \"vinfo\": must be add, info, or remove\n\
info: 0:\n\
prefix: 1:bad option \"var\": must be add, info, or remove\n\
vinfo2: 1:bad option \"vinfo\": must be add, info, or remove\n\
crossremove: 1:bad option \"vdelete\": must be add, info, or remove\n\
vinfo3: 1:bad option \"vinfo\": must be add, info, or remove\n\
modernadd: 0:\n\
legacyremove: 1:bad option \"vdelete\": must be add, info, or remove\n\
vinfo4: 1:bad option \"vinfo\": must be add, info, or remove\n\
all: 1:bad option \"variable\": must be add, info, or remove\n\
vinfoall: 1:bad option \"vinfo\": must be add, info, or remove\n\
infoall: 0:\n\
vivify: 00\n\
legacyfire: 0:1\n\
fired write\n\
modernfire: 0:1";

struct Vector {
    version: TclVersion,
    expected: &'static str,
    env: &'static str,
    tclsh: &'static [&'static str],
}

const VECTORS: &[Vector] = &[
    Vector {
        version: TclVersion::V8_4,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH84",
        tclsh: &["tclsh8.4"],
    },
    Vector {
        version: TclVersion::V8_5,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH85",
        tclsh: &["tclsh8.5"],
    },
    Vector {
        version: TclVersion::V8_6,
        expected: EXPECT_8X,
        env: "TCL_LSP_TCLSH86",
        tclsh: &["tclsh8.6"],
    },
    Vector {
        version: TclVersion::V9_0,
        expected: EXPECT_9X,
        env: "TCL_LSP_TCLSH90",
        tclsh: &["tclsh9.0"],
    },
    Vector {
        version: TclVersion::V9_1,
        expected: EXPECT_9X,
        env: "TCL_LSP_TCLSH91",
        tclsh: &["tclsh9.1"],
    },
];

#[test]
fn legacy_variable_trace_forms_follow_the_selected_release() {
    for vector in VECTORS {
        assert_eq!(
            vm_output(SCRIPT, vector.version),
            vector.expected,
            "{:?}",
            vector.version
        );
    }
}

#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut ran = 0;
    for vector in VECTORS {
        if let Some(actual) = tclsh_output(vector.env, vector.tclsh, SCRIPT) {
            assert_eq!(actual, vector.expected, "{:?}", vector.version);
            ran += 1;
        }
    }
    if ran == 0 {
        eprintln!("skipping: no versioned tclsh binaries found");
    }
}

/// The registry, not the VM, states the 9.0 boundary — so a spec edit moves
/// the runtime with it.
#[test]
fn the_registry_owns_the_release_boundary() {
    let registry = CommandRegistry::build_default();
    let spec = registry.get("trace").expect("trace is registered");
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let mask = DialectProfile::by_name(version.dialect_name()).availability_mask;
        for name in ["variable", "vdelete", "vinfo"] {
            assert!(
                spec.resolve_subcommand_for_dialect(name, mask).is_some(),
                "{version:?} should carry trace {name}"
            );
        }
    }
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let mask = DialectProfile::by_name(version.dialect_name()).availability_mask;
        for name in ["variable", "vdelete", "vinfo"] {
            assert!(
                spec.resolve_subcommand_for_dialect(name, mask).is_none(),
                "{version:?} should not carry trace {name}"
            );
        }
    }
}
