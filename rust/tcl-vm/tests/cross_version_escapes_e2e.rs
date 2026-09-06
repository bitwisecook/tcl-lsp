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

//! **The backslash-escape grammar moves with the release too.**
//!
//! `TclParseBackslash` changed once in the supported range, at 8.6 (TIP 388):
//! `\x` stopped swallowing every trailing hex digit and took at most two,
//! `\UHHHHHHHH` appeared, and an octal escape stopped taking a third digit once
//! the first two had reached `0x20`. 9.0 keeps 8.6's widths but raises
//! `TCL_UTF_MAX` to 4, so a decoded scalar past U+FFFF finally survives instead
//! of degrading to U+FFFD.
//!
//! The failure this guards is the same shape as the numeral suite's: nothing
//! errors. `\x4142` is a perfectly good escape under every release — it is just
//! `B` up to 8.5 and `A42` from 8.6, so a decoder pinned to the wrong release
//! silently produces a **different string of a different length** (issue #1479).
//! Both halves of an escape move together: its *value* and its *width*. A
//! consumer that measured it under one release and decoded it under another
//! would slice mid-escape, which is why the decoder computes both in one scan.
//!
//! Every vector prints only ASCII (`string length` and `scan … %c`) so the
//! byte-comparison against a real interpreter does not depend on the terminal
//! encoding a release happens to use for its output channel. The 8.6 and 9.0
//! columns are pinned against **tclsh 8.6.14 and 9.0.4** and are byte-compared
//! against a real interpreter whenever one is on `PATH` (`TCLSH86` / `TCLSH90`
//! override the binary names); the 8.4/8.5 column is read from
//! `tcl8.4.20/generic/tclParse.c:553-685` and `tcl8.5.19/generic/tclParse.c:839-975`,
//! which are identical here.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
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

/// Run `src` in the VM at `version`, returning its `puts` output.
///
/// Compiles *for* the release and then runs at it, exactly as `tclvm` does.
/// Both halves matter: a literal word's escapes are decoded at codegen, and
/// `subst`'s at run time, so pinning only one of the two leaves the other on
/// the 9.0 default.
fn vm_output(src: &str, version: TclVersion) -> String {
    let dialect = version.dialect_name();
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let ir = lower_to_ir(
        src,
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile()),
    );
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
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
    /// Tcl 8.4 and 8.5: pre-TIP-388 — unbounded `\x`, no `\U`, unguarded octal.
    want_8x: &'static str,
    /// Tcl 8.6: TIP 388's widths, but a UTF-16-internal string representation.
    want_86: &'static str,
    /// Tcl 9.0 and 9.1: TIP 388's widths with UCS-4 strings.
    want_90: &'static str,
}

/// Each vector prints `<length> <first code point>` so the claim covers both
/// the escape's width and its value, in ASCII the byte-comparison can trust.
const VECTORS: &[Vector] = &[
    // -- `\x`: TIP 388 capped it at two hex digits --
    Vector {
        name: "\\x41 reads the same everywhere",
        script: "set s \"\\x41\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 65",
        want_86: "1 65",
        want_90: "1 65",
    },
    Vector {
        name: "\\x4142 keeps the low byte of every hex digit until 8.6",
        script: "set s \"\\x4142\"\nputs \"[string length $s] [scan $s %c]\"\n",
        // 0x4142 truncated to a byte is 0x42 = `B`.
        want_8x: "1 66",
        want_86: "3 65",
        want_90: "3 65",
    },
    Vector {
        name: "\\x with no hex digit is a literal x everywhere",
        script: "set s \"\\xZZ\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "3 120",
        want_86: "3 120",
        want_90: "3 120",
    },
    // -- `\U`: TIP 388 introduced it; 9.0 gave it room above the BMP --
    Vector {
        name: "\\U is not an escape before 8.6",
        script: "set s \"\\U0001F600\"\nputs \"[string length $s] [scan $s %c]\"\n",
        // A literal `U` followed by eight digit characters.
        want_8x: "9 85",
        // 8.6's stock build is UTF-16-internal, so an astral scalar is U+FFFD.
        want_86: "1 65533",
        want_90: "1 128512",
    },
    Vector {
        name: "\\U41 is a short wide escape from 8.6",
        script: "set s \"\\U41\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "3 85",
        want_86: "1 65",
        want_90: "1 65",
    },
    // -- `\u`: unchanged across the whole range --
    Vector {
        name: "\\u00E9 reads the same everywhere",
        script: "set s \"\\u00E9\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 233",
        want_86: "1 233",
        want_90: "1 233",
    },
    // -- octal: the third digit's guard is 8.6's --
    Vector {
        name: "\\101 reads the same everywhere",
        script: "set s \"\\101\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 65",
        want_86: "1 65",
        want_90: "1 65",
    },
    Vector {
        name: "\\400 takes a third octal digit until 8.6",
        script: "set s \"\\400\"\nputs \"[string length $s] [scan $s %c]\"\n",
        // 0o400 truncated to a byte is NUL; from 8.6 it is `\40` plus `0`.
        want_8x: "1 0",
        want_86: "2 32",
        want_90: "2 32",
    },
    Vector {
        name: "\\777 takes a third octal digit until 8.6",
        script: "set s \"\\777\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 255",
        want_86: "2 63",
        want_90: "2 63",
    },
    Vector {
        name: "\\377 is in range on every release",
        script: "set s \"\\377\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 255",
        want_86: "1 255",
        want_90: "1 255",
    },
    // -- the line continuation is release-invariant --
    Vector {
        name: "line continuation collapses to one space everywhere",
        script: "set s \"a\\\n   b\"\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "3 97",
        want_86: "3 97",
        want_90: "3 97",
    },
    // -- the runtime `subst` path, not the compiled-literal one --
    Vector {
        name: "subst decodes under the emulated release",
        script: "set s [subst {\\x4142}]\nputs \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 66",
        want_86: "3 65",
        want_90: "3 65",
    },
    Vector {
        name: "subst -novariables -nocommands takes the same axis",
        script: "set s [subst -novariables -nocommands {\\400}]\n\
                 puts \"[string length $s] [scan $s %c]\"\n",
        want_8x: "1 0",
        want_86: "2 32",
        want_90: "2 32",
    },
];

#[test]
fn escape_grammar_follows_the_emulated_release() {
    for v in VECTORS {
        assert_eq!(
            vm_output(v.script, TclVersion::V8_4),
            v.want_8x,
            "[8.4] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_5),
            v.want_8x,
            "[8.5] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V8_6),
            v.want_86,
            "[8.6] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_0),
            v.want_90,
            "[9.0] {}",
            v.name
        );
        assert_eq!(
            vm_output(v.script, TclVersion::V9_1),
            v.want_90,
            "[9.1] {}",
            v.name
        );
    }
}

/// Byte-compare every vector against a real interpreter when one is installed.
/// Skips silently otherwise, so the suite stays useful on a machine with no
/// system Tcl — the pinned expectations above still run.
#[test]
fn vectors_match_real_tclsh_when_available() {
    let mut checked = 0usize;
    for v in VECTORS {
        if let Some(out) = tclsh_output("TCLSH85", &[], v.script) {
            assert_eq!(out, v.want_8x, "[tclsh8.5] {}", v.name);
            checked += 1;
        }
        if let Some(out) = tclsh_output("TCLSH86", &["tclsh8.6"], v.script) {
            assert_eq!(out, v.want_86, "[tclsh8.6] {}", v.name);
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

/// Two interpreters at different releases coexist: the escape grammar rides on
/// each VM's pinned dialect profile, not on process-wide state, so pinning the
/// second must not retune the first.
#[test]
fn a_second_interpreter_does_not_change_the_first_ones_grammar() {
    let script = "set s \"\\x4142\"\nputs \"[string length $s] [scan $s %c]\"\n";
    let older = vm_output(script, TclVersion::V8_5);
    let newer = vm_output(script, TclVersion::V9_0);
    assert_eq!(older, "1 66");
    assert_eq!(newer, "3 65");
    // …and again in the other order.
    assert_eq!(vm_output(script, TclVersion::V9_0), "3 65");
    assert_eq!(vm_output(script, TclVersion::V8_5), "1 66");
}
