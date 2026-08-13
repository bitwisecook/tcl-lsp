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

//! **The numeral grammar moves as one across every release.**
//!
//! Tcl's numeric-literal grammar changed twice in the supported range: `0o`/`0b`
//! arrive in 8.5, `0d` and `_` separators in 9.0, and octal-by-leading-zero is
//! retired *in* 9.0. Every consumer that reaches an integer conversion inherits
//! all of it — `expr`, `string is`, `incr`, `lindex`, `uplevel`'s level word —
//! because C routes them through the same `TclParseNumber`.
//!
//! The failure this guards is not a crash. Most numerals read identically under
//! every grammar, so a consumer pinned to the wrong release keeps working until
//! it meets one of the few that differ, and then returns a **valid number with
//! the wrong value** — `0755` is 493 up to 8.6 and 755 from 9.0, and nothing
//! errors either way. Six independent hand-rolled parsers had drifted apart in
//! this workspace before the single facility landed; this suite is what stops a
//! seventh, at the level where it is observable to a script.
//!
//! The vectors below are pinned against **tclsh 8.6.16 and 9.0.4** and are
//! byte-compared against a real interpreter whenever one is on `PATH`
//! (`TCLSH86` / `TCLSH90` override the binary names).
//!
//! See `docs/design/contracts/numeric-tower-and-expr-semantics.md`.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::TclVersion;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

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

struct CompilerSvc {
    registry: CommandRegistry,
    /// The compile target for any script the VM compiles at run time (`eval`,
    /// a proc body): the same release it is emulating.
    dialect: &'static str,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(
            src,
            &self.registry,
            tcl_lexer::LexerConfig::default(),
            self.dialect,
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// Run `src` in the VM at `version`, returning its `puts` output.
///
/// Compiles *for* the release and then runs at it, exactly as `tclvm` does.
/// Both halves matter and they are not interchangeable: constant folding
/// happens at codegen, so a dialect-blind compile bakes in the default
/// grammar's answer and `set_runtime_version` cannot undo it afterwards —
/// `puts "v: [expr {0755 + 1}]"` would fold to 756 and then run on an 8.4
/// interpreter. The dialect is a compile target, not a runtime switch.
fn vm_output(src: &str, version: TclVersion) -> String {
    let dialect = version.dialect_name();
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry, tcl_lexer::LexerConfig::default(), dialect);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
        dialect,
    }));
    vm.set_runtime_version(version);
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
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(src.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else {
            continue;
        };
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}

/// One script and what each release must print.
struct Vector {
    name: &'static str,
    script: &'static str,
    /// Tcl 8.4: `0x` and leading-zero octal only.
    want_84: &'static str,
    /// Tcl 8.5 / 8.6: adds `0o` and `0b`; leading zero is still octal.
    want_8x: &'static str,
    /// Tcl 9.0 / 9.1: adds `0d` and `_`; leading zero is decimal.
    want_90: &'static str,
}

/// A vector that expects a rejection prints only the `catch` **code**, never the
/// message: the claim under test is *which release* rejects the spelling, and
/// pinning the full `ParseExpr` diagnosis here would duplicate the message
/// coverage that `opcode_c_parity` already owns and make this suite brittle to
/// unrelated wording changes. Vectors that expect a value print the value.
const VECTORS: &[Vector] = &[
    // -- radix prefixes, by the release that introduced each --
    Vector {
        name: "hex is universal",
        script: "puts [expr {0x1f}]\n",
        want_84: "31",
        want_8x: "31",
        want_90: "31",
    },
    Vector {
        name: "0o octal arrives in 8.5",
        script: "puts [catch {expr {0o17}}]\n",
        want_84: "1",
        want_8x: "0",
        want_90: "0",
    },
    Vector {
        name: "0b binary arrives in 8.5",
        script: "puts [catch {expr {0b101}}]\n",
        want_84: "1",
        want_8x: "0",
        want_90: "0",
    },
    Vector {
        name: "0d decimal arrives in 9.0",
        script: "puts [catch {expr {0d9}}]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "0",
    },
    Vector {
        name: "_ separators arrive in 9.0",
        script: "puts [catch {expr {1_0}}]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "0",
    },
    // -- the dangerous row: a valid number with a *different value* --
    Vector {
        name: "leading zero is octal until 9.0",
        script: "puts [expr {0755}]\n",
        want_84: "493",
        want_8x: "493",
        want_90: "755",
    },
    Vector {
        name: "leading zero, const-folded subexpression",
        script: "puts [expr {0755 + 1}]\n",
        want_84: "494",
        want_8x: "494",
        want_90: "756",
    },
    Vector {
        name: "leading zero, interpolated (the fold-policy path)",
        script: "puts \"v: [expr {0755 + 1}]\"\n",
        want_84: "v: 494",
        want_8x: "v: 494",
        want_90: "v: 756",
    },
    Vector {
        name: "08 is an invalid octal digit until 9.0",
        script: "puts [catch {expr {08}}]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "0",
    },
    // -- every consumer that reaches an integer conversion inherits the rule --
    Vector {
        name: "incr amount",
        script: "set n 0\nincr n 010\nputs $n\n",
        want_84: "8",
        want_8x: "8",
        want_90: "10",
    },
    Vector {
        name: "list index",
        script: "puts [lindex {a b c d e f g h i j k l} 010]\n",
        want_84: "i",
        want_8x: "i",
        want_90: "k",
    },
    Vector {
        name: "end-relative list index",
        script: "puts [lindex {a b c d e f g h i j k l} end-010]\n",
        want_84: "d",
        want_8x: "d",
        want_90: "b",
    },
    Vector {
        name: "string is integer",
        script: "puts [string is integer 08][string is integer 1_0]\n",
        want_84: "00",
        want_8x: "00",
        want_90: "11",
    },
    Vector {
        name: "string is double takes a radix integer on every release",
        script: "puts [string is double 0x1f]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "1",
    },
    // -- a radix-invalid numeral is never a number on any release --
    Vector {
        name: "0o8 is not a numeral anywhere",
        script: "puts [catch {expr {0o8}} r]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "1",
    },
    // -- an empty numeral errors; it must never abort the process --
    Vector {
        name: "empty operand errors rather than panicking",
        script: "set e {}\nputs [catch {expr {$e + 1}} r]\n",
        want_84: "1",
        want_8x: "1",
        want_90: "1",
    },
];

#[test]
fn numeral_grammar_follows_the_emulated_release() {
    for v in VECTORS {
        assert_eq!(vm_output(v.script, TclVersion::V8_4), v.want_84, "[8.4] {}", v.name);
        assert_eq!(vm_output(v.script, TclVersion::V8_5), v.want_8x, "[8.5] {}", v.name);
        assert_eq!(vm_output(v.script, TclVersion::V8_6), v.want_8x, "[8.6] {}", v.name);
        assert_eq!(vm_output(v.script, TclVersion::V9_0), v.want_90, "[9.0] {}", v.name);
        assert_eq!(vm_output(v.script, TclVersion::V9_1), v.want_90, "[9.1] {}", v.name);
    }
}

/// The level word is read by `Tcl_GetIntFromObj`, so it inherits the grammar
/// too — kept separate because it needs a call chain deep enough that the
/// divergent value is *in range* on both sides. C reports `bad level` both for
/// a word it cannot parse and for a level past the top of the stack, so a
/// shallow chain hides the difference behind a shared error.
#[test]
fn uplevel_level_word_follows_the_emulated_release() {
    let src = "proc f0 {} {f1}\nproc f1 {} {f2}\nproc f2 {} {f3}\nproc f3 {} {f4}\n\
               proc f4 {} {f5}\nproc f5 {} {f6}\nproc f6 {} {f7}\nproc f7 {} {f8}\n\
               proc f8 {} {f9}\nproc f9 {} {f10}\n\
               proc f10 {} { puts [uplevel 010 {info level}] }\nf0\n";
    // 11 frames deep: `010` is 8 up (→ level 3) until 9.0, and 10 up (→ 1) after.
    for (version, want) in [
        (TclVersion::V8_4, "3"),
        (TclVersion::V8_5, "3"),
        (TclVersion::V8_6, "3"),
        (TclVersion::V9_0, "1"),
        (TclVersion::V9_1, "1"),
    ] {
        assert_eq!(vm_output(src, version), want, "{version:?}");
    }
}

/// Byte-compare every vector against a real interpreter when one is installed.
/// Skips silently otherwise, so the suite stays useful on a machine with no
/// system Tcl — the pinned expectations above still run.
#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut checked = 0usize;
    for v in VECTORS {
        if let Some(out) = tclsh_output("TCLSH86", &["tclsh8.6"], v.script) {
            assert_eq!(out, v.want_8x, "[tclsh8.6] {}", v.name);
            checked += 1;
        }
        if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], v.script) {
            assert_eq!(out, v.want_90, "[tclsh9.0] {}", v.name);
            checked += 1;
        }
    }
    if checked == 0 {
        eprintln!("no system tclsh found — pinned expectations still verified");
    }
}
